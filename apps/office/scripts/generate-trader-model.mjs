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
  { skin: '#e7b998', hair: '#49352f', coat: '#287b72', shirt: '#f5eee1', pants: '#273746' },
  { skin: '#9c684d', hair: '#29252c', coat: '#b6654d', shirt: '#f1e6d2', pants: '#303741' },
  { skin: '#f1c9a8', hair: '#bd7952', coat: '#596f9c', shirt: '#f7f2e9', pants: '#35404d' },
  { skin: '#704a3d', hair: '#171a20', coat: '#c49a54', shirt: '#e9f0e7', pants: '#283c3a' },
  { skin: '#d6a184', hair: '#382b27', coat: '#725b8c', shirt: '#f2ebdf', pants: '#384053' },
  { skin: '#f0c8aa', hair: '#5a5146', coat: '#4c7d9c', shirt: '#fff4df', pants: '#303d46' },
  { skin: '#916044', hair: '#17191d', coat: '#71954d', shirt: '#f2eadd', pants: '#283344' },
  { skin: '#efd0b7', hair: '#765444', coat: '#bd6756', shirt: '#f3f0e7', pants: '#333344' },
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
    colored(new THREE.CapsuleGeometry(0.26, 0.37, 3, 7), palette.coat, [0, 1.12, 0], [0, 0, Math.PI]),
    colored(new THREE.CapsuleGeometry(0.16, 0.24, 2, 6), palette.shirt, [0, 1.16, -0.235], [0, 0, Math.PI]),
    colored(new THREE.CylinderGeometry(0.065, 0.07, 0.2, 6), palette.skin, [0, 1.46, 0]),
    colored(new THREE.DodecahedronGeometry(0.24, 1), palette.skin, [0, 1.75, -0.025], [0.12, 0, 0], [0.92, 1.05, 0.9]),
    colored(new THREE.SphereGeometry(0.25, 12, 8, 0, Math.PI * 2, 0, Math.PI * 0.43), palette.hair, [0, 1.84, 0.005], [Math.PI, 0, 0], [1.04, 0.98, 1]),
    colored(new THREE.ConeGeometry(0.045, 0.11, 5), palette.skin, [0, 1.73, -0.235], [Math.PI / 2, 0, 0]),
  ]
  for (const side of [-1, 1]) {
    torso.push(
      colored(new THREE.SphereGeometry(0.037, 8, 6), palette.skin, [side * 0.235, 1.75, -0.005]),
      colored(new THREE.SphereGeometry(0.023, 8, 6), '#242628', [side * 0.082, 1.78, -0.22]),
      colored(new THREE.CapsuleGeometry(0.115, 0.38, 2, 6), palette.pants, [side * 0.14, 0.52, 0], [0.03, 0, side * 0.025]),
      colored(new THREE.BoxGeometry(0.18, 0.1, 0.27), '#252d36', [side * 0.14, 0.12, -0.08]),
    )
  }
  // Two facial hairstyles and a softly varied hair silhouette across palette variants.
  if (index % 3 === 1 || index % 3 === 2) torso.push(colored(new THREE.BoxGeometry(0.1, 0.13, 0.12), palette.hair, [0, 1.58, -0.205]))
  const bodyGeometry = mergeGeometries(torso, false); bodyGeometry.computeVertexNormals()
  body.add(new THREE.Mesh(bodyGeometry, mat)); root.add(body)
  for (const side of [-1, 1]) {
    const arm = new THREE.Group(); arm.name = side < 0 ? 'Arm_L' : 'Arm_R'; arm.position.set(side * 0.31, 1.34, -0.015)
    const parts = [
      colored(new THREE.CapsuleGeometry(0.105, 0.27, 2, 6), palette.coat, [0, -0.2, 0], [0, 0, -side * 0.15]),
      colored(new THREE.CapsuleGeometry(0.074, 0.23, 2, 6), palette.shirt, [side * 0.045, -0.43, -0.035], [0, 0, side * 0.12]),
      colored(new THREE.SphereGeometry(0.075, 8, 6), palette.skin, [side * 0.07, -0.59, -0.08]),
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
console.log(`Wrote ${Buffer.byteLength(result)} byte low-poly trader library`)
