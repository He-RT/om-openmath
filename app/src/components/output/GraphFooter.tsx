import { useState, type ReactNode } from 'react';
import type { Messages } from '../../i18n';
export function GraphFooter({ t, renderStatus, information, exports, status }: {
  t: Messages; renderStatus?: ((actions: ReactNode) => ReactNode) | undefined;
  information: ReactNode; exports: ReactNode;
  status?: string | null;
}) {
  const [open, setOpen] = useState(false);
  const actions = <><button aria-expanded={open} onClick={() => setOpen(value => !value)}>{t.graphInfo}</button>{exports}</>;
  return <>
    {renderStatus ? renderStatus(actions) : <div className="graph-footer">{actions}</div>}
    <div className="graph-information" hidden={!open}>{information}</div>
    {status && <p className="graph-export-status" role="status">{status}</p>}
  </>;
}
