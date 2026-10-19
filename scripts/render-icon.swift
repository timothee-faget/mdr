// Renders an SVG to a transparent 1024×1024 PNG with WebKit, which keeps the
// system fonts and the alpha channel (QuickLook flattens onto white).
// Usage: swift scripts/render-icon.swift <in.svg> <out.png>
import AppKit
import WebKit

let svg = try! String(contentsOfFile: CommandLine.arguments[1], encoding: .utf8)
let output = CommandLine.arguments[2]

final class Renderer: NSObject, WKNavigationDelegate {
    var window: NSWindow!
    func start() {
        let webView = WKWebView(frame: NSRect(x: 0, y: 0, width: 1024, height: 1024))
        webView.setValue(false, forKey: "drawsBackground")
        webView.navigationDelegate = self
        window = NSWindow(contentRect: webView.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = webView
        webView.loadHTMLString("<html><body style='margin:0;background:transparent'>\(svg)</body></html>", baseURL: nil)
    }
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            let config = WKSnapshotConfiguration()
            config.snapshotWidth = 512 // points; the 2× backing scale gives 1024 px
            webView.takeSnapshot(with: config) { image, _ in
                let rep = NSBitmapImageRep(data: image!.tiffRepresentation!)!
                try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: output))
                NSApp.terminate(nil)
            }
        }
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
let renderer = Renderer()
renderer.start()
app.run()
