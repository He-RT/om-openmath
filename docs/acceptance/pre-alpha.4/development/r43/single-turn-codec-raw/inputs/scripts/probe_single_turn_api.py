#!/usr/bin/env python3
"""Explicit authorized private credential -> Rust-built request -> actual SSE -> Rust codec.

Development smoke only. No Registry import, Pi task, native URLSession or automatic retries.
"""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import stat
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', required=True, type=Path)
    parser.add_argument('--fixture', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    info = args.config.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        parser.error('config must be an owned private regular file')
    config = json.loads(args.config.read_text(encoding='utf-8'))
    endpoint = config['base_url'].rstrip('/')
    url = urllib.parse.urlsplit(endpoint)
    if url.scheme != 'https' or not url.hostname or url.username or url.password or url.query or url.fragment:
        parser.error('plain HTTPS endpoint required')
    models, key = config['models'], config['api_key']
    if not isinstance(models, list) or not 1 <= len(models) <= 2 or not all(isinstance(m, str) and 0 < len(m) <= 128 for m in models):
        parser.error('one or two explicit model names required')
    if not isinstance(key, str) or not key or '\r' in key or '\n' in key:
        parser.error('invalid credential')

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, request, fp, code, message, headers, new_url):
            raise urllib.error.HTTPError(request.full_url, code, 'redirect rejected', headers, fp)

    def codec(frame):
        executed = subprocess.run([str(args.fixture.resolve())], input=json.dumps(frame, ensure_ascii=False).encode(), capture_output=True, timeout=10, check=False)
        if executed.returncode:
            raise ValueError('codec rejected response')
        if key.encode() in executed.stdout:
            raise ValueError('credential echo rejected')
        return json.loads(executed.stdout)

    def probe(model):
        started = time.monotonic()
        input_message = {'record_type': 'canonical_agent_message', 'message_version': 1, 'message_id': 'probe-user', 'role': 'user', 'agent_turn_id': 'probe-turn', 'task_id': 'probe-task', 'parts': [{'kind': 'text', 'part_id': 'text', 'text': 'Return only the single ASCII digit equal to 2+2.'}]}
        frame = {'request': {'binding': {'provider_ref': 'explicit-test-provider', 'route_ref': 'explicit-test-route', 'model_ref': model, 'adapter_version': 'single-turn-1', 'protocol': 'chat_completions'}, 'api_model': model, 'messages': [{'canonical': input_message, 'replay': None}], 'results': [], 'tools': [], 'max_output_tokens': 64, 'temperature': None, 'stream': True}, 'message_id': 'probe-assistant', 'task_id': 'probe-task', 'turn_id': 'probe-turn', 'status': 200, 'response': None}
        report = {'model': model}
        try:
            body = codec(frame)['body']
            report['request_body'] = body
            request = urllib.request.Request(endpoint + '/chat/completions', data=json.dumps(body).encode(), headers={'Authorization': 'Bearer ' + key, 'Accept': 'text/event-stream', 'Content-Type': 'application/json'})
            with urllib.request.build_opener(NoRedirect()).open(request, timeout=45) as response:
                raw = response.read(1024 * 1024 + 1)
                report['http_status'] = response.status
            if len(raw) > 1024 * 1024 or key.encode() in raw:
                raise ValueError('bounded response/credential echo rejected')
            frame['response'] = raw.decode('utf-8')
            # Complete exact untrusted provider wire; no authorization header or credential.
            report['response_sse'] = frame['response']
            actual = codec(frame)
            if actual['body'] != body:
                raise ValueError('request snapshot changed')
            report['decoded'] = actual['result']
            result = actual['result']
            text = ''.join(p['text'] for p in result['message']['parts'] if p['kind'] == 'text')
            report['matched'] = result['finish'] == 'complete' and text.strip() == '4'
        except urllib.error.HTTPError as error:
            report['http_status'] = error.code
        except Exception as error:
            report['error_type'] = type(error).__name__
        report['elapsed_ms'] = round((time.monotonic() - started) * 1000, 3)
        return report

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(probe, models))
    report = {'endpoint': endpoint, 'scope': 'authorized development actual Chat SSE/Rust codec; no native transport/Registry/Pi or release gate claim', 'models': results}
    serialized = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
    if key in serialized:
        raise ValueError('credential echo rejected')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(serialized, encoding='utf-8')
    print(json.dumps({'models': [{k: r[k] for k in ['model', 'http_status', 'matched', 'error_type', 'elapsed_ms'] if k in r} for r in results]}))
    return 0 if all(r.get('matched') for r in results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
