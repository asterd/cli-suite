import { readFileSync } from "node:fs";

export class Service {
  run(name: string): string {
    return readFileSync(name, "utf8");
  }
}

export function processRequest(id: number): string {
  return `ok:${id}`;
}
