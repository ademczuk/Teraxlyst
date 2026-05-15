// Top-level dialog that listens for `teraxlyst:mcp-prompt` events and renders
// the right field component based on the `kind` discriminator. On submit,
// calls the `mcp_prompt_response` Tauri command.
//
// Wired into App.tsx as a sibling component to the existing dialogs.

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ConfirmField } from "./fields/ConfirmField";
import { EditTextField } from "./fields/EditTextField";
import { MultiSelectField } from "./fields/MultiSelectField";
import { ReorderField } from "./fields/ReorderField";
import { SingleSelectField } from "./fields/SingleSelectField";
import type { PromptField } from "./types";
import { useMcpPrompts } from "./useMcpPrompts";

function renderField(
  field: PromptField,
  onSubmit: (value: unknown) => void,
) {
  switch (field.kind) {
    case "multi-select":
      return (
        <MultiSelectField
          label={field.label}
          options={field.options}
          min={field.min}
          max={field.max}
          onSubmit={onSubmit}
        />
      );
    case "single-select":
      return (
        <SingleSelectField
          label={field.label}
          options={field.options}
          onSubmit={onSubmit}
        />
      );
    case "reorder":
      return (
        <ReorderField
          label={field.label}
          items={field.items}
          onSubmit={onSubmit}
        />
      );
    case "edit-text":
      return (
        <EditTextField
          label={field.label}
          defaultValue={field.default}
          placeholder={field.placeholder}
          onSubmit={onSubmit}
        />
      );
    case "confirm":
      return <ConfirmField label={field.label} onSubmit={onSubmit} />;
  }
}

export function PromptForUserInputDialog() {
  const { current, submit } = useMcpPrompts();

  const open = current != null;

  return (
    <Dialog open={open}>
      <DialogContent
        showCloseButton={false}
        onEscapeKeyDown={(e) => e.preventDefault()}
        onPointerDownOutside={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>Agent request</DialogTitle>
          {current && <DialogDescription>{current.prompt_text}</DialogDescription>}
        </DialogHeader>
        {current &&
          renderField(current.field, (value) => submit(current.id, value))}
      </DialogContent>
    </Dialog>
  );
}
