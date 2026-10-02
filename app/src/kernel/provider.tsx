import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { createKernelClient, type KernelClient } from './index';
const Context = createContext<{ client: KernelClient | null; error: string | null }>({ client: null, error: null });
export function KernelProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<{ client: KernelClient | null; error: string | null }>({ client: null, error: null });
  useEffect(() => {
    let disposed = false; let client: KernelClient | undefined;
    void createKernelClient().then(value => { client = value; if (disposed) value.dispose(); else setState({ client: value, error: null }); }).catch(() => { if (!disposed) setState({ client: null, error: 'Kernel initialization failed' }); });
    return () => { disposed = true; client?.dispose(); };
  }, []);
  return <Context.Provider value={state}>{children}</Context.Provider>;
}
export function useKernel() { return useContext(Context); }
