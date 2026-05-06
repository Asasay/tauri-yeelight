import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";

type Props = {
  status: string;
};

export function ResponseCard({ status }: Props) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Device response</CardTitle>
        <CardDescription>Last response from your light.</CardDescription>
      </CardHeader>
      <CardContent>
        <pre className="max-h-96 overflow-auto rounded-lg border border-slate-800 bg-slate-950 p-4 text-xs text-slate-200">
          {status}
        </pre>
      </CardContent>
    </Card>
  );
}