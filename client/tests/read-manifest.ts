import { LAYER_VERSION, consoleManifest, onThisLayer } from "bmo-ui/shared/console-manifest";

// The service's manifest read through bmo-ui's own reading of the contract (bmo-ui's `docs/console-bundles.md`,
// "What a service owes"): the manifest on standard input; on standard output the manifest as read, its
// defaults filled, beside the layer this bmo-ui is and whether the manifest stands on it; or the reading's
// fault on standard error and an exit of 1. The service's own test hands it what its route answers.

try {
  const manifest = consoleManifest(JSON.parse(await Bun.stdin.text()) as unknown);

  console.log(JSON.stringify({ manifest, layer: LAYER_VERSION, onThisLayer: onThisLayer(manifest) }));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
