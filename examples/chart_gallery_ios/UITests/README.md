# Chart Gallery UI tests

This hostless XCUITest bundle launches the already-installed `dev.ely.chartgallery` app. It does not build, install, or embed the app.

Generate and run the test bundle on the simulator that already contains the app:

```sh
cd <ely-repository>/examples/chart_gallery_ios/UITests
export SIMULATOR_UDID=<installed-app-simulator-udid>
xcodegen generate --spec project.yml
xcodebuild \
  -project ChartGalleryUITests.xcodeproj \
  -scheme ChartGalleryUITests \
  -destination "platform=iOS Simulator,id=$SIMULATOR_UDID" \
  test
```

The test drives the app window with XCTest's native two-touch `pinch` and slow coordinate drag. It resets the app between gestures, keeps before/after screenshots, and requires at least 1,000 changed pixels in the central screen region after each gesture. Inspect the screenshots to confirm that the chart changed rather than the surrounding content.
