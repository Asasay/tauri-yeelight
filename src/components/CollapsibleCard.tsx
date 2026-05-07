import { useRef, useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";

type Props = {
  title: string;
  defaultFolded?: boolean;
  children: React.ReactNode;
};

export function CollapsibleCard({ title, defaultFolded = false, children }: Props) {
  const [folded, setFolded] = useState(defaultFolded);
  const contentRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState<number | "auto">(folded ? 0 : "auto");

  const toggle = () => {
    if (folded) {
      setHeight(contentRef.current?.scrollHeight ?? "auto");
      setTimeout(() => setHeight("auto"), 300);
    } else {
      setHeight(contentRef.current?.scrollHeight ?? 0);
      setTimeout(() => setHeight(0), 10);
    }
    setFolded(!folded);
  };

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between space-y-0 py-2">
        <CardTitle className="text-base">{title}</CardTitle>
        <button onClick={toggle} className="p-1 hover:text-slate-300 transition-colors">
          {folded ? <ChevronDown className="h-4 w-4" /> : <ChevronUp className="h-4 w-4" />}
        </button>
      </CardHeader>
      <div
        ref={contentRef}
        className="overflow-hidden transition-all duration-300 ease-in-out"
        style={{ height: height === "auto" ? "auto" : `${height}px` }}
      >
        <CardContent>{children}</CardContent>
      </div>
    </Card>
  );
}