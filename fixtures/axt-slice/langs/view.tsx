import React from "react";
import { readFileSync } from "node:fs";

export const View = ({ label }: { label: string }) => {
  return <section>{label}</section>;
};

export function loadLabel(path: string): string {
  return readFileSync(path, "utf8");
}
