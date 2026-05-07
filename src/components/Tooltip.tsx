import { ReactNode } from "react";

type Props = {
  text: string;
  children: ReactNode;
};

export function Tooltip({ text, children }: Props) {
  return (
    <div className="relative group w-full">
      {children}
      <span className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-2 py-1 whitespace-nowrap rounded bg-slate-800 text-xs text-slate-200 opacity-0 group-hover:opacity-100 transition-opacity z-50 pointer-events-none">
        {text}
      </span>
    </div>
  );
}