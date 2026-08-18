// QR decoding from a captured photo (licence QR scan). Pure JS via `jsqr`:
// no native plugin, works in the iOS WKWebView and on desktop browsers. The
// photo comes from an `<input type="file" capture>` so iOS shows its native
// camera UI (NSCameraUsageDescription is set in Info.plist).
import jsQR from "jsqr";

/** Draw `file` on a canvas at `maxSide` resolution and return its pixels. */
async function imageDataFromFile(
  file: File,
  maxSide: number,
): Promise<ImageData | null> {
  const url = URL.createObjectURL(file);
  try {
    const img = await new Promise<HTMLImageElement>((resolve, reject) => {
      const el = new Image();
      el.onload = () => resolve(el);
      el.onerror = () => reject(new Error("unreadable image"));
      el.src = url;
    });
    const scale = Math.min(1, maxSide / Math.max(img.width, img.height));
    const width = Math.max(1, Math.round(img.width * scale));
    const height = Math.max(1, Math.round(img.height * scale));
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) return null;
    context.drawImage(img, 0, 0, width, height);
    return context.getImageData(0, 0, width, height);
  } catch {
    return null;
  } finally {
    URL.revokeObjectURL(url);
  }
}

/** Decode the QR code in a photo, or null. Tries two resolutions: phone
 * photos are huge and jsQR often locks on better after a downscale. */
export async function decodeQrFromFile(file: File): Promise<string | null> {
  for (const maxSide of [1600, 800]) {
    const pixels = await imageDataFromFile(file, maxSide);
    if (!pixels) continue;
    const hit = jsQR(pixels.data, pixels.width, pixels.height);
    const text = hit?.data.trim();
    if (text) return text;
  }
  return null;
}
