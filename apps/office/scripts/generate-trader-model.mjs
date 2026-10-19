import * as THREE from 'three'
import { mergeGeometries, mergeVertices } from 'three/examples/jsm/utils/BufferGeometryUtils.js'
import { GLTFExporter } from 'three/examples/jsm/exporters/GLTFExporter.js'
import { mkdir, writeFile } from 'node:fs/promises'

// GLTFExporter only needs FileReader when exporting image textures. The model
// deliberately uses vertex colors and no external textures, so this small
// polyfill is enough to make the deterministic asset build work in Node.
globalThis.FileReader ??= class FileReader {
  async readAsArrayBuffer(blob) { this.result = await blob.arrayBuffer(); this.onloadend?.() }
  async readAsDataURL(blob) { this.result = `data:${blob.type};base64,${Buffer.from(await blob.arrayBuffer()).toString('base64')}`; this.onloadend?.() }
}

const palettes = [
  { skin: '#e7b998', hair: '#49352f', coat: '#263d4b', tie: '#a27754', shirt: '#f5eee1', pants: '#273746' },
  { skin: '#9c684d', hair: '#29252c', coat: '#4c3837', tie: '#c0a16a', shirt: '#f1e6d2', pants: '#303741' },
  { skin: '#f1c9a8', hair: '#bd7952', coat: '#40576c', tie: '#7c9e9b', shirt: '#f7f2e9', pants: '#35404d' },
  { skin: '#704a3d', hair: '#171a20', coat: '#4c5049', tie: '#b19053', shirt: '#e9f0e7', pants: '#283c3a' },
  { skin: '#d6a184', hair: '#382b27', coat: '#493d50', tie: '#b16b5a', shirt: '#f2ebdf', pants: '#384053' },
  { skin: '#f0c8aa', hair: '#5a5146', coat: '#354c55', tie: '#aa8452', shirt: '#fff4df', pants: '#303d46' },
  { skin: '#916044', hair: '#17191d', coat: '#35463e', tie: '#b16e58', shirt: '#f2eadd', pants: '#283344' },
  { skin: '#efd0b7', hair: '#765444', coat: '#51413e', tie: '#829a88', shirt: '#f3f0e7', pants: '#333344' },
]

const mat = new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.8, metalness: 0.02 })
function colored(geometry, hex, position, rotation = [0, 0, 0], scale = [1, 1, 1]) {
  if (geometry.index) geometry = geometry.toNonIndexed()
  geometry = mergeVertices(geometry, 1e-4)
  geometry.translate(...position)
  geometry.rotateX(rotation[0]); geometry.rotateY(rotation[1]); geometry.rotateZ(rotation[2])
  geometry.scale(...scale)
  const c = new THREE.Color(hex), colors = new Float32Array(geometry.attributes.position.count * 3)
  for (let i = 0; i < colors.length; i += 3) { colors[i] = c.r; colors[i + 1] = c.g; colors[i + 2] = c.b }
  geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3))
  return geometry
}
function model(palette, index) {
  const root = new THREE.Group(); root.name = `trader-${index}`
  const body = new THREE.Group(); body.name = 'Body'
  const torso = [
    colored(new THREE.CapsuleGeometry(0.275, 0.4, 8, 16), palette.coat, [0, 1.12, 0], [0, 0, Math.PI]),
    colored(new THREE.CapsuleGeometry(0.15, 0.25, 6, 12), palette.shirt, [0, 1.16, -0.24], [0, 0, Math.PI]),
    colored(new THREE.BoxGeometry(0.19, 0.1, 0.07), palette.shirt, [0, 1.39, -0.26]),
    colored(new THREE.BoxGeometry(0.065, 0.31, 0.035), palette.tie, [0, 1.17, -0.345]),
    colored(new THREE.CylinderGeometry(0.018, 0.062, 0.12, 8), palette.tie, [0, 0.96, -0.34]),
    colored(new THREE.CylinderGeometry(0.065, 0.07, 0.2, 12), palette.skin, [0, 1.46, 0]),
    colored(new THREE.SphereGeometry(0.235, 24, 18), palette.skin, [0, 1.75, -0.025], [0.08, 0, 0], [0.9, 1.08, 0.92]),
    colored(new THREE.SphereGeometry(0.243, 24, 16, 0, Math.PI * 2, 0, Math.PI * 0.43), palette.hair, [0, 1.85, 0.005], [Math.PI, 0, 0], [1.04, 0.98, 1]),
    colored(new THREE.ConeGeometry(0.035, 0.09, 10), palette.skin, [0, 1.72, -0.235], [Math.PI / 2, 0, 0]),
  ]
  for (const side of [-1, 1]) {
    torso.push(
      colored(new THREE.SphereGeometry(0.04, 14, 10), palette.skin, [side * 0.224, 1.75, -0.01]),
      colored(new THREE.SphereGeometry(0.023, 12, 8), '#f4eee5', [side * 0.078, 1.786, -0.227]),
      colored(new THREE.SphereGeometry(0.014, 12, 8), '#242628', [side * 0.078, 1.786, -0.247]),
      colored(new THREE.CapsuleGeometry(0.115, 0.39, 6, 12), palette.pants, [side * 0.14, 0.52, 0], [0.03, 0, side * 0.025]),
      colored(new THREE.BoxGeometry(0.19, 0.1, 0.29), '#20262a', [side * 0.14, 0.12, -0.08]),
      colored(new THREE.SphereGeometry(0.034, 10, 8), '#9f835b', [side * 0.07, 1.04, -0.274]),
    )
  }
  // Two facial hairstyles and a softly varied hair silhouette across palette variants.
  if (index % 3 === 1 || index % 3 === 2) torso.push(colored(new THREE.BoxGeometry(0.1, 0.13, 0.12), palette.hair, [0, 1.58, -0.205]))
  const bodyGeometry = mergeGeometries(torso, false); bodyGeometry.computeVertexNormals()
  body.add(new THREE.Mesh(bodyGeometry, mat)); root.add(body)
  for (const side of [-1, 1]) {
    const arm = new THREE.Group(); arm.name = side < 0 ? 'Arm_L' : 'Arm_R'; arm.position.set(side * 0.31, 1.34, -0.015)
    const parts = [
      colored(new THREE.CapsuleGeometry(0.105, 0.28, 6, 12), palette.coat, [0, -0.2, 0], [0, 0, -side * 0.15]),
      colored(new THREE.CapsuleGeometry(0.074, 0.23, 5, 10), palette.shirt, [side * 0.045, -0.43, -0.035], [0, 0, side * 0.12]),
      colored(new THREE.SphereGeometry(0.075, 14, 10), palette.skin, [side * 0.07, -0.59, -0.08]),
    ]
    const armGeometry = mergeGeometries(parts, false); armGeometry.computeVertexNormals()
    arm.add(new THREE.Mesh(armGeometry, mat)); root.add(arm)
  }
  return root
}

const stage = new THREE.Group(); stage.name = 'TraderLibrary'
for (const [index, palette] of palettes.entries()) stage.add(model(palette, index))
const result = await new GLTFExporter().parseAsync(stage, { binary: true, trs: true, onlyVisible: true })
await mkdir(new URL('../public/models/', import.meta.url), { recursive: true })
await writeFile(new URL('../public/models/traders.glb', import.meta.url), Buffer.from(result))
console.log(`Wrote ${Buffer.byteLength(result)} byte trader model library`)
