#!/bin/sh
# Reuse gpui-mobile's UIKit host; this directory owns only the Rust gallery adapter.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ely=$(CDPATH= cd -- "$here/../.." && pwd)
mobile=$(CDPATH= cd -- "$ely/../gpui-mobile" && pwd)
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-1.97.1}
export IPHONEOS_DEPLOYMENT_TARGET=16.0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$mobile/example/target"}

rustup run "$RUSTUP_TOOLCHAIN" cargo rustc --manifest-path "$here/Cargo.toml" --locked --lib --target aarch64-apple-ios-sim -j 1
sdk=$(xcrun --sdk iphonesimulator --show-sdk-path)
host="$mobile/example/ios"
app="$here/ios-build/ElyChartGallery.app"
mkdir -p "$app"
cp "$host/Info.plist" "$app/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :UIDeviceFamily array' -c 'Add :UIDeviceFamily:0 integer 1' -c 'Add :UIDeviceFamily:1 integer 2' "$app/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIdentifier dev.ely.chartgallery' "$app/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleExecutable ElyChartGallery' "$app/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleName Ely Chart Gallery' "$app/Info.plist"
xcrun ibtool --compile "$app/LaunchScreen.storyboardc" "$host/LaunchScreen.storyboard" --target-device iphone --target-device ipad --minimum-deployment-target 16.0
xcrun --sdk iphonesimulator clang -target arm64-apple-ios16.0-simulator -isysroot "$sdk" \
  -fobjc-arc -DUSE_GPUI_RUST=1 -Dgpui_ios_run_demo=gpui_ios_run_chart_gallery -I"$host" "$host/main.m" \
  -Wl,-force_load,"$CARGO_TARGET_DIR/aarch64-apple-ios-sim/debug/libely_chart_gallery_ios.a" \
  -lc++ -liconv -lresolv -lz -framework UIKit -framework Metal -framework MetalKit \
  -framework QuartzCore -framework CoreGraphics -framework CoreText -framework CoreFoundation \
  -framework Foundation -framework Security -framework SystemConfiguration -framework Network \
  -framework AudioToolbox -framework WebKit -framework CoreMedia -framework AVFoundation \
  -framework LocalAuthentication -framework QuickLook -framework ImageIO -framework UniformTypeIdentifiers \
  -framework Speech -framework AppTrackingTransparency -framework Contacts -framework EventKit \
  -framework StoreKit -framework CoreMotion -framework MediaPlayer -framework MapKit \
  -framework CoreLocation -framework Photos -framework PhotosUI -framework CoreVideo \
  -framework CoreBluetooth -framework CoreHaptics -framework UserNotifications -ObjC -o "$app/ElyChartGallery"
codesign --force --sign - "$app"
if [ "${1:-}" != "--no-run" ]; then
  simulator=${SIMULATOR_UDID:-booted}
  xcrun simctl install "$simulator" "$app"
  xcrun simctl launch "$simulator" dev.ely.chartgallery
fi
printf '%s\n' "$app"
