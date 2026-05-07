import { CollapsibleCard } from "./CollapsibleCard";

type Props = {
  status: string;
};

export function ResponseCard({ status }: Props) {
  return (
    <CollapsibleCard title="Device response" defaultFolded={false}>
      <pre className="max-h-96 overflow-auto rounded-lg border border-slate-800 bg-slate-950 p-4 text-xs text-slate-200 scrollbar-hide">
        {status}
      </pre>
    </CollapsibleCard>
  );
}