import CoreGraphics
import ImageIO
import XCTest
import UIKit

final class ChartGalleryUITests: XCTestCase {
    private let app = XCUIApplication(bundleIdentifier: "dev.ely.chartgallery")

    func testPinchAndSlowDragChangeTheChartGallery() throws {
        try exerciseGestures()
    }

    func testTabletPortraitGestures() throws {
        guard UIDevice.current.userInterfaceIdiom == .pad else { throw XCTSkip("iPad only") }
        try exerciseGestures(portrait: true)
    }

    private func exerciseGestures(portrait: Bool = false) throws {
        app.launch()
        if portrait { XCUIDevice.shared.orientation = .portrait }

        let window = app.windows.firstMatch
        XCTAssertTrue(window.waitForExistence(timeout: 10), "Chart Gallery window did not appear")
        if portrait {
            XCTAssertLessThan(window.frame.width, window.frame.height, "iPad did not rotate to portrait")
        }
        if window.frame.height > window.frame.width {
            window.coordinate(withNormalizedOffset: CGVector(dx: 0.30, dy: 0.17)).tap()
        }

        let before = window.screenshot()
        attach(before, named: "before-gesture")

        // XCTest sends a native two-touch gesture to the installed app's window.
        window.pinch(withScale: 4.0, velocity: 1.0)
        let afterPinch = window.screenshot()
        attach(afterPinch, named: "after-pinch")

        // Start the brush at the full range, independent of the pinch result.
        app.terminate()
        app.launch()
        XCTAssertTrue(window.waitForExistence(timeout: 10))
        if window.frame.height > window.frame.width {
            window.coordinate(withNormalizedOffset: CGVector(dx: 0.30, dy: 0.17)).tap()
        }
        let beforeDrag = window.screenshot()
        attach(beforeDrag, named: "before-drag")

        let dragY = portrait ? 0.42 : 0.60
        let start = window.coordinate(withNormalizedOffset: CGVector(dx: 0.20, dy: dragY))
        let end = window.coordinate(withNormalizedOffset: CGVector(dx: 0.70, dy: dragY))
        start.press(
            forDuration: 0.2,
            thenDragTo: end,
            withVelocity: .slow,
            thenHoldForDuration: 0.2
        )

        let after = window.screenshot()
        attach(after, named: "after-drag")

        guard let pinchPixels = changedPixelCount(from: before, to: afterPinch),
              let dragPixels = changedPixelCount(from: beforeDrag, to: after) else {
            return XCTFail("Could not decode comparable screenshots")
        }
        XCTAssertGreaterThan(pinchPixels, 1_000, "Pinch did not produce an observable pixel change")
        XCTAssertGreaterThan(dragPixels, 1_000, "Drag did not produce an observable pixel change")
    }

    private func attach(_ screenshot: XCUIScreenshot, named name: String) {
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func changedPixelCount(from before: XCUIScreenshot, to after: XCUIScreenshot) -> Int? {
        guard let beforePixels = rgbaPixels(before), let afterPixels = rgbaPixels(after),
              beforePixels.width == afterPixels.width, beforePixels.height == afterPixels.height else {
            return nil
        }

        let rowBytes = beforePixels.width * 4
        return stride(from: rowBytes * beforePixels.height / 4,
                      to: rowBytes * beforePixels.height * 9 / 10, by: 4)
            .reduce(into: 0) { count, offset in
                if beforePixels.bytes[offset..<(offset + 4)] != afterPixels.bytes[offset..<(offset + 4)] {
                    count += 1
                }
            }
    }

    private func rgbaPixels(_ screenshot: XCUIScreenshot) -> (width: Int, height: Int, bytes: [UInt8])? {
        guard let source = CGImageSourceCreateWithData(screenshot.pngRepresentation as CFData, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
            return nil
        }

        let width = image.width
        let height = image.height
        var bytes = [UInt8](repeating: 0, count: width * height * 4)
        let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)
            .union(.byteOrder32Big)
        guard let context = CGContext(
            data: &bytes,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: bitmapInfo.rawValue
        ) else {
            return nil
        }

        context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
        return (width, height, bytes)
    }
}
