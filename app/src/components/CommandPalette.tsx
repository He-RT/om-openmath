import { Command } from "cmdk";
import type { Messages } from "../i18n";
export function CommandPalette({
  open,
  onOpen,
  t,
  commands,
}: {
  open: boolean;
  onOpen: (open: boolean) => void;
  t: Messages;
  commands: { label: string; run: () => void; shortcut?: string }[];
}) {
  return (
    <Command.Dialog
      open={open}
      onOpenChange={onOpen}
      label={t.commands}
      className="command-dialog"
    >
      <Command.Input placeholder={t.search} />
      <Command.List>
        <Command.Empty>…</Command.Empty>
        {commands.map((command) => (
          <Command.Item
            key={command.label}
            onSelect={() => {
              command.run();
              onOpen(false);
            }}
          >
            <span>{command.label}</span>
            {command.shortcut && <kbd>{command.shortcut}</kbd>}
          </Command.Item>
        ))}
      </Command.List>
    </Command.Dialog>
  );
}
