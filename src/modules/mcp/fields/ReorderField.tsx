import { useState } from "react";

interface Props {
  label: string;
  items: string[];
  onSubmit: (value: string[]) => void;
}

// Minimal reorder: up/down buttons, no drag-and-drop. drag-drop is a v1.1
// polish item; the brief asks for "reorder", and up/down is the smallest
// thing that satisfies that.
export function ReorderField({ label, items, onSubmit }: Props) {
  const [order, setOrder] = useState<string[]>(items);

  const move = (index: number, delta: number) => {
    const next = index + delta;
    if (next < 0 || next >= order.length) return;
    const copy = order.slice();
    const tmp = copy[index];
    copy[index] = copy[next];
    copy[next] = tmp;
    setOrder(copy);
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="text-sm font-medium">{label}</div>
      <ul className="flex flex-col gap-1">
        {order.map((item, i) => (
          <li
            key={`${item}-${i}`}
            className="flex items-center gap-2 rounded-md border px-2 py-1"
          >
            <span className="flex-1">{item}</span>
            <button
              type="button"
              onClick={() => move(i, -1)}
              disabled={i === 0}
              className="rounded px-2 py-0.5 text-xs disabled:opacity-30"
            >
              up
            </button>
            <button
              type="button"
              onClick={() => move(i, 1)}
              disabled={i === order.length - 1}
              className="rounded px-2 py-0.5 text-xs disabled:opacity-30"
            >
              down
            </button>
          </li>
        ))}
      </ul>
      <div className="flex justify-end">
        <button
          type="button"
          onClick={() => onSubmit(order)}
          className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground"
        >
          Submit
        </button>
      </div>
    </div>
  );
}
