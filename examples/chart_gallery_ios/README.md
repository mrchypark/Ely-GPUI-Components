# Ely chart gallery for iOS Simulator

This is a thin UIKit host for the current shared `examples/mobile_gallery`. It does not copy gallery pages or chart stories. It opens on Charts and exposes every chart story through the existing story controls.

## Build, install, and open

On an Apple Silicon Mac with Xcode and a booted iOS simulator:

```sh
cd <ely-repository>
rustup target add --toolchain 1.97.1 aarch64-apple-ios-sim
sh examples/chart_gallery_ios/ios-simulator.sh
```

Set `SIMULATOR_UDID` to target another simulator. Use `--no-run` to create and sign the app without installing or launching it:

```sh
SIMULATOR_UDID=<udid> sh examples/chart_gallery_ios/ios-simulator.sh --no-run
```

The script creates `ios-build/ElyChartGallery.app`, installs it as `dev.ely.chartgallery`, and starts it by default. It uses the local Ely source tree, local `gpui-mobile` vendor patches, and GPUI Mobile's existing `example/target` cache unless `CARGO_TARGET_DIR` overrides it. The gallery materializes its bundled files in the app cache at runtime and receives iOS safe-area and keyboard-inset events.

## Scope and verification

The host opens the complete shared gallery at Charts; use the existing story buttons to exercise each chart story.
