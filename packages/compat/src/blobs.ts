// A recording's payloads (the texture and buffer data of a WebGL context) are blobs: each content is written once, to
// the file of the render's blob directory named by the SHA-256 of its bytes, and the recording names it by ref
// (packages/cli/skills/kinetograph-markup/references/SCENE.md, "glsl"). The directory is the host's: it makes it when a
// render opens and removes it when the render ends.

import { createHash } from 'node:crypto';
import { renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { threadId } from 'node:worker_threads';
import type { BlobSink } from '@kinetographdev/scene';

/** A sink that writes each content once to `dir`, as the file named by its hash. */
export function fileBlobSink(dir: string): BlobSink {
  const written = new Set<string>();
  return {
    dir,
    put(bytes) {
      const hex = createHash('sha256').update(bytes).digest('hex');
      if (!written.has(hex)) {
        // Content-addressed: any writer of this name writes these bytes, so writers that race (render workers) are safe.
        const partial = join(dir, `${hex}.${process.pid}-${threadId}.partial`);
        writeFileSync(partial, bytes);
        renameSync(partial, join(dir, hex));
        written.add(hex);
      }
      return `sha256:${hex}`;
    },
  };
}
