import { useState } from "react";
import { Input } from "@/components/ui/input";

interface Props {
  label: string;
  defaultValue?: string | null;
  placeholder?: string | null;
  onSubmit: (value: string) => void;
}

export function EditTextField({
  label,
  defaultValue,
  placeholder,
  onSubmit,
}: Props) {
  const [value, setValue] = useState<string>(defaultValue ?? "");

  return (
    <div className="flex flex-col gap-3">
      <div className="text-sm font-medium">{label}</div>
      <Input
        autoFocus
        value={value}
        placeholder={placeholder ?? undefined}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") onSubmit(value);
        }}
      />
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => onSubmit(value)}
          className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground"
        >
          Submit
        </button>
      </div>
    </div>
  );
}
