// Generates the octopus PWA icon set: a mint "> ▋" prompt on near-black.
// Rasterizes SVG → PNG via headless chromium (no image-lib dependency).
import { writeFileSync, mkdirSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const MINT = '#3ef5c4', BG = '#06070a'
const PUBLIC = new URL('../public/', import.meta.url).pathname

// scale: fraction of the 512 viewBox the glyph group occupies (1 = the coords below).
// A smaller scale (maskable) shrinks the glyph toward the center for the safe zone.
function svg(scale = 1) {
  // Base glyph coords are tuned for scale=1 (standard icon).
  const glyph = `
    <polyline points="176,196 256,256 176,316" fill="none" stroke="${MINT}"
      stroke-width="34" stroke-linecap="round" stroke-linejoin="round"/>
    <rect x="292" y="214" width="46" height="84" rx="6" fill="${MINT}"/>`
  // Scale the glyph group about the canvas center (256,256).
  const tx = 256 * (1 - scale), ty = 256 * (1 - scale)
  return `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
    <rect width="512" height="512" fill="${BG}"/>
    <g filter="url(#g)" transform="translate(${tx} ${ty}) scale(${scale})">${glyph}</g>
    <defs><filter id="g" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="6" result="b"/><feMerge>
      <feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>
  </svg>`
}

const STANDARD = svg(1)          // > ▋ filling ~55%
const MASKABLE = svg(0.72)       // shrunk into the maskable safe zone

// Rasterize an SVG string to a PNG of the given pixel size via headless chromium.
function raster(svgStr, size, outName) {
  const html = `<!doctype html><html><head><meta charset="utf-8">
    <style>html,body{margin:0;padding:0;background:${BG}}
    svg{display:block;width:${size}px;height:${size}px}</style></head>
    <body>${svgStr}</body></html>`
  const htmlPath = join(tmpdir(), `octo-icon-${size}-${outName}.html`)
  const pngPath = join(PUBLIC, outName)
  writeFileSync(htmlPath, html)
  execFileSync('chromium', [
    '--headless', '--disable-gpu', '--no-sandbox', '--hide-scrollbars',
    '--force-device-scale-factor=1', `--window-size=${size},${size}`,
    `--screenshot=${pngPath}`, '--default-background-color=00000000',
    `file://${htmlPath}`,
  ], { stdio: 'inherit' })
  console.log('wrote', pngPath)
}

mkdirSync(PUBLIC, { recursive: true })
raster(STANDARD, 192, 'icon-192.png')
raster(STANDARD, 512, 'icon-512.png')
raster(MASKABLE, 512, 'icon-512-maskable.png')
raster(STANDARD, 180, 'apple-touch-icon.png')
writeFileSync(join(PUBLIC, 'favicon.svg'), STANDARD)
console.log('wrote', join(PUBLIC, 'favicon.svg'))
