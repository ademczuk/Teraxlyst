import { useState } from "react";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";

interface Props {
  label: string;
  options: string[];
  onSubmit: (value: string) => void;
}

export function SingleSelectField({ label, options, onSubmit }: Props) {
  const [value, setValue] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-3">
      <div className="text-sm font-medium">{label}</div>
      <RadioGroup value={value ?? ""} onValueChange={setValue}>
        {options.map((option) => (
          <Label
            key={option}
            className="flex items-center gap-2 cursor-pointer"
          >
            <RadioGroupItem value={option} />
            <span>{option}</span>
          </Label>
        ))}
      </RadioGroup>
      <div className="flex justify-end">
        <button
          type="button"
          disabled={value == null}
          onClick={() => value != null && onSubmit(value)}
          className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
        >
          Submit
        </button>
      </div>
    </div>
  );
}
