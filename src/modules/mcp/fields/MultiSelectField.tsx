import { useState } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";

interface Props {
  label: string;
  options: string[];
  min?: number | null;
  max?: number | null;
  onSubmit: (value: string[]) => void;
}

export function MultiSelectField({ label, options, min, max, onSubmit }: Props) {
  const [selected, setSelected] = useState<Set<string>>(new Set());

  const toggle = (option: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(option)) {
        next.delete(option);
      } else {
        if (max != null && next.size >= max) return prev;
        next.add(option);
      }
      return next;
    });
  };

  const count = selected.size;
  const disabled = (min != null && count < min) || (max != null && count > max);

  return (
    <div className="flex flex-col gap-3">
      <div className="text-sm font-medium">{label}</div>
      <div className="flex flex-col gap-2">
        {options.map((option) => (
          <Label
            key={option}
            className="flex items-center gap-2 cursor-pointer"
          >
            <Checkbox
              checked={selected.has(option)}
              onCheckedChange={() => toggle(option)}
            />
            <span>{option}</span>
          </Label>
        ))}
      </div>
      <div className="flex justify-end">
        <button
          type="button"
          disabled={disabled}
          onClick={() => onSubmit(Array.from(selected))}
          className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
        >
          Submit
        </button>
      </div>
    </div>
  );
}
