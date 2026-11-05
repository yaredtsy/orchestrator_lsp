import { blake3 } from "@noble/hashes/blake3";

// 02-05 §1 (VS-D50): id = base32( blake3( repo_relative_path + "::" + qualname + "#" + occurrence )[..16] ), 26 chars.
// Spike choice: lowercase RFC 4648 alphabet, no padding.
const ALPHABET = "abcdefghijklmnopqrstuvwxyz234567";

export function base32(bytes: Uint8Array): string {
  let bits = 0, value = 0, out = "";
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += ALPHABET[(value << (5 - bits)) & 31];
  return out;
}

export function deriveId(relPath: string, qualname: string, occurrence: number): string {
  const input = `${relPath}::${qualname}#${occurrence}`;
  return base32(blake3(new TextEncoder().encode(input), { dkLen: 16 }));
}

export function textHash(text: string): string {
  return Buffer.from(blake3(new TextEncoder().encode(text))).toString("hex");
}

export interface OutlineNode {
  name: string;
  type: "class" | "function";
  id?: string | null;
  position: { line: number; column: number; end_line: number; end_column: number };
  has_syntax_error?: boolean;
  children: OutlineNode[];
}

export interface IdRequest { line: number; column: number; id: string; qualname: string }

/** Walks the outline in source order; every def with no ID gets the hash-derived one. */
export function idsFor(relPath: string, nodes: OutlineNode[]): IdRequest[] {
  const seen = new Map<string, number>();
  const out: IdRequest[] = [];
  const walk = (ns: OutlineNode[], prefix: string) => {
    for (const n of ns) {
      const qualname = prefix ? `${prefix}.${n.name}` : n.name;
      const occurrence = (seen.get(qualname) ?? 0) + 1;
      seen.set(qualname, occurrence);
      if (!n.id) {
        out.push({ line: n.position.line, column: n.position.column, id: deriveId(relPath, qualname, occurrence), qualname });
      }
      walk(n.children, qualname);
    }
  };
  walk(nodes, "");
  return out;
}
