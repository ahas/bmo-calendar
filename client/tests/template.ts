import { readFileSync } from "node:fs";

import { parse } from "vue/compiler-sfc";

import type { SFCDescriptor } from "vue/compiler-sfc";

// A screen's template as the Vue compiler reads it: its elements by tag, an attribute by its name whether
// written plain or bound, and the expression a listener runs.

type TemplateNode = NonNullable<NonNullable<SFCDescriptor["template"]>["ast"]>["children"][number];
export type Element = Extract<TemplateNode, { tag: string }>;

/** The compiled descriptor of a single-file component. */
export function component(path: string): SFCDescriptor {
  return parse(readFileSync(path, "utf8")).descriptor;
}

/** Every element under a node, in document order. */
export function elements(node: { children?: unknown }): Element[] {
  const children = Array.isArray(node.children) ? (node.children as TemplateNode[]) : [];

  return children.flatMap((child) => ("tag" in child ? [child, ...elements(child)] : elements(child)));
}

/** Every element of a component's template. */
export function templateElements(path: string): Element[] {
  const ast = component(path).template?.ast;

  return ast ? elements(ast) : [];
}

/** An attribute's value, written plain, or the source of its binding. */
export function attribute(element: Element, name: string): string | undefined {
  for (const prop of element.props) {
    if (!("exp" in prop) && prop.name === name) return prop.value?.content;
    if ("exp" in prop && prop.name === "bind" && prop.arg && "content" in prop.arg && prop.arg.content === name) return prop.exp?.loc.source;
  }

  return undefined;
}

/** The source of what a listener for an event runs, where the element listens for it. */
export function listener(element: Element, event: string): string | undefined {
  for (const prop of element.props) {
    if ("exp" in prop && prop.name === "on" && prop.arg && "content" in prop.arg && prop.arg.content === event) return prop.exp?.loc.source ?? "";
  }

  return undefined;
}
