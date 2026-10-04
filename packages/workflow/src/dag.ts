import { CheaperError } from "@cheaper/core";
import type { DependencyEdge } from "./types.js";

export function compileDag(jobIds: readonly string[], dependencies: readonly DependencyEdge[]): {
  order: string[]; edges: DependencyEdge[];
} {
  const ids = [...jobIds].sort();
  const outgoing = new Map(ids.map((id) => [id, new Set<string>()]));
  const indegree = new Map(ids.map((id) => [id, 0]));
  for (const { from, to } of dependencies) {
    if (!outgoing.has(from) || !outgoing.has(to)) throw new CheaperError("INVALID_REQUEST", `unknown dependency: ${from} -> ${to}`);
    if (from === to) throw new CheaperError("INVALID_REQUEST", `self dependency: ${from}`);
    const children = outgoing.get(from) as Set<string>;
    if (!children.has(to)) {
      children.add(to);
      indegree.set(to, (indegree.get(to) as number) + 1);
    }
  }
  const edges = ids.flatMap((from) => [...(outgoing.get(from) as Set<string>)].sort().map((to) => ({ from, to })));
  const ready = ids.filter((id) => indegree.get(id) === 0);
  const order: string[] = [];
  while (ready.length) {
    const id = ready.shift() as string;
    order.push(id);
    for (const child of outgoing.get(id) as Set<string>) {
      const remaining = (indegree.get(child) as number) - 1;
      indegree.set(child, remaining);
      if (remaining === 0) ready.push(child);
    }
    ready.sort();
  }
  if (order.length !== ids.length) throw new CheaperError("INVALID_REQUEST", "workflow dependency cycle", {
    details: { jobs: ids.filter((id) => (indegree.get(id) as number) > 0) },
  });
  return { order, edges };
}
