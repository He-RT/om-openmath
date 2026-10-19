import {
  EventHub,
  type KernelClient,
  type Request,
  type Response,
  type Event,
} from "./client";
import type { Envelope } from "./generated/Envelope";
import type { KernelConfig } from "./generated/KernelConfig";
import type { NotebookFile } from "./generated/NotebookFile";
import { LlmDriver } from "./llmDriver";
import { sourceMutation, mutatesSource, mergeConfig } from "./mirror";
import { restoreConfig } from "../state/browserSettings";
import type {MathWork,MathResult} from '../agent/hostTypes';
import {prepareProfileModel} from '../agent/profileModel';
type Packet =
  | { response: Envelope<Response|{type:'agent_math';value:MathResult}>; events: Envelope<Event>[] }
  | { type: "ready" }
  | { type: "error"; id?: number; message: string };
export interface WorkerPort {
  postMessage(message: unknown): void;
  terminate(): void;
  onmessage: ((event: MessageEvent<Packet>) => void) | null;
  onerror: ((event: ErrorEvent) => void) | null;
}
export class WasmClient implements KernelClient {
  readonly kind = "wasm" as const;
  private hub = new EventHub();
  private worker!: WorkerPort;
  private boot!: Promise<void>;
  private rejectBoot: ((error: Error) => void) | undefined;
  private readyPromise: Promise<void>;
  private pending = new Map<
    number,
    { resolve: (response: unknown) => void; reject: (error: Error) => void }
  >();
  private nextId = 1;
  private generation = 0;
  private closed = false;
  private failure: Error | undefined;
  private config: KernelConfig | undefined;
  private file: NotebookFile = { version: 1, title: "", cells: [] };
  private revision = 0;
  private restoreConfig: KernelConfig | undefined;
  private configRevision = 0;
  private driver: LlmDriver;
  constructor(
    private factory: () => WorkerPort = () =>
      new Worker(new URL("./worker.ts", import.meta.url), { type: "module" }),
    fetcher?: typeof fetch,
  ) {
    this.driver = new LlmDriver((req) => this.raw(req), fetcher);
    this.startWorker();
    this.readyPromise = this.initialize();
    void this.readyPromise.catch(() => {});
  }
  get ready() {
    return this.readyPromise;
  }
  onEvent(callback: (event: Event) => void) {
    return this.hub.onEvent(callback);
  }
  private startWorker() {
    this.failure = undefined;
    const worker = this.factory();
    this.worker = worker;
    const generation = ++this.generation;
    this.boot = new Promise((resolve, reject) => {
      this.rejectBoot = reject;
      worker.onerror = () => {
        if (generation !== this.generation) return;
        const error = new Error("Kernel worker failed");
        this.failure = error;
        reject(error);
        this.rejectAll(error);
        worker.terminate();
      };
      worker.onmessage = (event) => {
        if (generation !== this.generation || this.closed) return;
        const packet = event.data;
        if ("type" in packet) {
          if (packet.type === "ready") resolve();
          else if (packet.id !== undefined) {
            this.pending.get(packet.id)?.reject(new Error(packet.message));
            this.pending.delete(packet.id);
          } else {
            const error = new Error(packet.message);
            this.failure = error;
            reject(error);
            this.rejectAll(error);
          }
          return;
        }
        for (const event of packet.events) {
          if (event.id !== 0) continue;
          this.driver.event(event.body);
          if (event.body.type !== "llm_http") this.hub.emit(event.body);
        }
        this.pending.get(packet.response.id)?.resolve(packet.response.body);
        this.pending.delete(packet.response.id);
      };
    });
    const config = this.restoreConfig ?? this.config;
    worker.postMessage({
      type: "init",
      ...(config ? { config: JSON.stringify(config) } : {}),
    });
  }
  private async initialize() {
    await this.boot;
    const response = await this.raw({ type: "get_config" });
    if (response.type !== "config")
      throw new Error("Kernel configuration unavailable");
    this.config = mergeConfig(response.config, this.config);
    const remembered = restoreConfig(this.config);
    if (remembered) {
      const applied = await this.raw({
        type: "set_config",
        config: remembered,
      });
      if (applied.type !== "error") this.config = remembered;
    }
  }
  private async raw(request: Request): Promise<Response> {
    return this.send(id => ({ type: 'request', id, envelope: JSON.stringify({ id, body: request }) }));
  }
  private async send<T=Response>(packet: (id: number) => unknown): Promise<T> {
    if (this.closed) throw new Error("Kernel client disposed");
    await this.boot;
    if (this.failure) throw this.failure;
    if (!Number.isSafeInteger(this.nextId))
      throw new Error("Kernel request ID exhausted");
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve:value=>resolve(value as T), reject });
      try {
        this.worker.postMessage(packet(id));
      } catch {
        this.pending.delete(id);
        reject(new Error("Kernel request could not be sent"));
      }
    });
  }
  async agentMath(work:MathWork):Promise<MathResult>{
    await this.readyPromise;
    const result=await this.send<{type:'agent_math';value:MathResult}|{type:'error';message:string}>(id=>({type:'agent-math',id,source:JSON.stringify(work)}));
    if(result.type==='error')throw new Error(result.message);
    return result.value;
  }
  async prepareAgentModel(name:string){await this.readyPromise;const profile=this.config?.llm.profiles.find(profile=>profile.name===name);if(!profile)throw new Error('Configured model is unavailable');return prepareProfileModel(profile,this.configRevision);}
  async commitSourceFile(expected: NotebookFile, desired: NotebookFile): Promise<Response> {
    await this.readyPromise;
    const revision = ++this.revision;
    const response = await this.send(id => ({ type: 'commit-source', id, expected: JSON.stringify(expected), desired: JSON.stringify(desired) }));
    if (response.type === 'notebook_state' && this.revision === revision) this.file = structuredClone(response.state.file);
    return response;
  }
  async request(request: Request): Promise<Response> {
    await this.readyPromise;
    if (request.type === "llm_cancel") this.driver.cancel(request.request_id);
    const previous = this.file;
    const changesSource = mutatesSource(request);
    if (changesSource) this.file = sourceMutation(this.file, request);
    const revision = changesSource ? ++this.revision : this.revision;
    const configRevision =
      request.type === "set_config" ? ++this.configRevision : undefined;
    if (request.type === "set_config")
      this.restoreConfig = mergeConfig(
        request.config,
        this.restoreConfig ?? this.config,
      );
    const response = await this.raw(request);
    if (response.type === "error" && changesSource && this.revision === revision)
      this.file = previous;
    if (request.type === "set_config") {
      if (response.type !== "error")
        this.config = mergeConfig(request.config, this.config);
      if (configRevision === this.configRevision)
        this.restoreConfig = undefined;
    }
    if (response.type === "llm_started" && response.http) {
      const route =
        request.type === "llm_test_profile"
          ? request.profile
          : request.type === "llm_translate"
            ? this.config?.llm.translate
            : request.type === "llm_explain"
              ? this.config?.llm.explain
              : request.type === "llm_complete"
                ? this.config?.llm.complete
                : request.type === "llm_fix_error"
                  ? this.config?.llm.fix
                  : this.config?.llm.chat;
      const timeout =
        request.type === "llm_test_profile" && request.config
          ? request.config.timeout_ms
          : (this.config?.llm.profiles.find((p) => p.name === route)
              ?.timeout_ms ?? 60_000);
      this.driver.start(response.request_id, response.http, timeout);
      return { ...response, http: null };
    }
    return response;
  }
  async interrupt() {
    if (this.closed) throw new Error("Kernel client disposed");
    const error = new Error("Computation interrupted; kernel restarted");
    this.driver.dispose();
    this.worker.terminate();
    this.rejectBoot?.(error);
    this.rejectAll(error);
    const file = structuredClone(this.file);
    this.startWorker();
    this.readyPromise = (async () => {
      await this.boot;
      const response = await this.raw({ type: "load_notebook", file });
      if (response.type === "error") throw new Error(response.message);
      const restored = await this.raw({ type: "restore_definitions" });
      if (restored.type !== "notebook_state")
        throw new Error("Kernel recovery failed");
      if (this.restoreConfig) {
        this.config = this.restoreConfig;
        this.restoreConfig = undefined;
      }
    })();
    void this.readyPromise.catch(() => {});
    await this.readyPromise;
  }
  private rejectAll(error: Error) {
    for (const request of this.pending.values()) request.reject(error);
    this.pending.clear();
  }
  dispose() {
    if (this.closed) return;
    const error = new Error("Kernel client disposed");
    this.closed = true;
    this.driver.dispose();
    this.worker.terminate();
    this.rejectBoot?.(error);
    this.rejectAll(error);
    this.hub.clear();
  }
}
