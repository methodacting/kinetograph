// Fork only (methodacting/kinetograph): renames the packages motionactor installs to `@motionactor/kinetograph-<name>`,
// all at one version, for the fleet registry. Each `@kinetographdev/*` or `kinetograph` dependency becomes an npm alias of
// the renamed package at that version, so a consumer keeps importing `@kinetographdev/*` and installs nothing from public
// npm; a peer on one of them becomes that exact version (the consumer's alias of the same name satisfies it).
//
// usage: node scripts/ma-alias.mjs <version>   (rewrites the manifests in place; CI only, never committed)
import fs from 'node:fs';

const REGISTRY = 'https://runactor-npm.methodactor.workers.dev';

/** The directories published, and each one's name after `@motionactor/kinetograph-`. */
const PUBLISHED = {
  'crates/node': 'native',
  'packages/scene': 'scene',
  'packages/compat': 'compat',
  'packages/react': 'react',
  'packages/core': 'core',
  'packages/audio': 'audio',
  'packages/video-matting': 'video-matting',
  'packages/studio': 'studio',
  'packages/cli': 'cli',
};

const version = process.argv[2];
// A prerelease identifier of digits only must not start with 0 (semver), so a short sha like 0123abcd is fine and 01234567 is not.
if (!/^\d+\.\d+\.\d+-main\.(?!0\d*\.)[0-9a-f]{7,12}\.\d+$/.test(version ?? '')) {
  console.error(`ma-alias: version must look like 0.4.0-main.<upstream short sha>.<build>, got ${version}`);
  process.exit(1);
}

const read = (dir) => JSON.parse(fs.readFileSync(`${dir}/package.json`, 'utf8'));
const renamed = new Map(Object.entries(PUBLISHED).map(([dir, short]) => [read(dir).name, `@motionactor/kinetograph-${short}`]));
const ours = (name) => name.startsWith('@kinetographdev/') || name === 'kinetograph';

const problems = [];
for (const dir of Object.keys(PUBLISHED)) {
  const pkg = read(dir);
  const from = pkg.name;
  pkg.name = renamed.get(from);
  pkg.version = version;
  pkg.repository = { type: 'git', url: 'git+https://github.com/methodacting/kinetograph.git', directory: dir };
  pkg.publishConfig = { registry: REGISTRY };
  delete pkg.optionalDependencies;
  for (const field of ['dependencies', 'peerDependencies']) {
    for (const name of Object.keys(pkg[field] ?? {})) {
      if (!ours(name)) continue;
      const to = renamed.get(name);
      if (to === undefined) problems.push(`${from} ${field} ${name}: not published by this script`);
      else pkg[field][name] = field === 'dependencies' ? `npm:${to}@${version}` : version;
    }
  }
  fs.writeFileSync(`${dir}/package.json`, `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`${from} -> ${pkg.name}@${version}`);
}
if (problems.length) {
  console.error(problems.join('\n'));
  process.exit(1);
}
