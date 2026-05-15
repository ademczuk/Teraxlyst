interface Props {
  label: string;
  onSubmit: (value: boolean) => void;
}

export function ConfirmField({ label, onSubmit }: Props) {
  return (
    <div className="flex flex-col gap-3">
      <div className="text-sm">{label}</div>
      <div className="flex justify-end gap-2">
        <button
          type="button"
          onClick={() => onSubmit(false)}
          className="rounded-md border px-3 py-1.5 text-sm"
        >
          No
        </button>
        <button
          type="button"
          onClick={() => onSubmit(true)}
          className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground"
        >
          Yes
        </button>
      </div>
    </div>
  );
}
