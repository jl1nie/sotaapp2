/**
 * 山頂の簡易地図を SVG に描くための関数。
 *
 * 出どころ: sotalive-monorepo の apps/fle/src/lib/summitMap.ts から、
 * projector・pathOf・PARK_STYLES・parkCodesOf・parkStyleOf・labelPointIn・withoutPrefecture を写した
 * (FLEonline の MapSheet と同じ見え方にするため)。FLEonline 側を直したら、ここも合わせる。
 * 違い: 管理画面の下地の地図には judge_park・peak の feature もあるので、型を広げ、
 * parkCodesOf は judge_park の公園番号も数える。
 */

export type Position = [number, number]
export type Ring = Position[]

export type Geometry =
  | { type: 'Point'; coordinates: Position }
  | { type: 'LineString'; coordinates: Position[] }
  | { type: 'MultiLineString'; coordinates: Position[][] }
  | { type: 'Polygon'; coordinates: Ring[] }
  | { type: 'MultiPolygon'; coordinates: Ring[][] }

export interface MuniPota {
  code: string
  name?: string
  coverage: 'all' | 'part'
}

/** kind: zone / border / muni / park / area / summit のほか、判定用の judge_park / peak */
export interface FeatureProps {
  kind: string
  key?: string
  name?: string
  code?: string
  alt?: number
  pota?: MuniPota[]
  [k: string]: unknown
}

export interface Feature {
  type: 'Feature'
  properties: FeatureProps
  geometry: Geometry
}

/** [西, 南, 東, 北] */
export type BBox = [number, number, number, number]

export interface SummitMap {
  type?: string
  summit?: string
  bbox: BBox
  attribution?: string
  features: Feature[]
  [k: string]: unknown
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null

/** 地図の形を確かめる(bbox と features)。崩れていれば null */
export function asSummitMap(raw: unknown): SummitMap | null {
  if (!isObj(raw)) return null
  const bbox = raw.bbox
  if (!Array.isArray(bbox) || bbox.length !== 4 || !bbox.every((v) => typeof v === 'number')) return null
  if (!Array.isArray(raw.features)) return null
  const features = raw.features.filter(
    (f): f is Feature => isObj(f) && isObj(f.properties) && isObj(f.geometry) && typeof f.properties.kind === 'string'
  )
  return { ...raw, bbox: bbox as BBox, features }
}

/** 市区町村名から都道府県を外す(山頂のコードで分かる。読み上げる欄の幅を取らない)。郡名は残す */
export const withoutPrefecture = (name: string): string => name.replace(/^(東京都|北海道|(?:京都|大阪)府|.{2,3}県)/, '')

function polygonsOf(g: Geometry): Ring[][] {
  if (g.type === 'Polygon') return [g.coordinates]
  if (g.type === 'MultiPolygon') return g.coordinates
  return []
}

/**
 * 経緯度を、地図の SVG の座標(幅 width、上が北)に変える。狭い範囲なので、緯度の縮みだけを直した平面で描く。
 * 返す height は bbox の縦横比から決まる
 */
export function projector(bbox: BBox, width: number) {
  const [w, s, e, n] = bbox
  const k = Math.cos((((s + n) / 2) * Math.PI) / 180)
  const scale = width / ((e - w) * k)
  const height = (n - s) * scale
  const xy = ([lon, lat]: Position): Position => [(lon - w) * k * scale, (n - lat) * scale]
  /** 1 m が何単位か(縮尺の棒や、GPS の誤差の円に使う) */
  const perMeter = scale / 111320
  return { width, height, xy, perMeter }
}

/** 多角形・線を SVG の path の d にする */
export function pathOf(g: Geometry, xy: (p: Position) => Position): string {
  const line = (pts: Position[], close: boolean) =>
    pts.length === 0
      ? ''
      : 'M' + pts.map((p) => xy(p).map((v) => v.toFixed(1)).join(',')).join('L') + (close ? 'Z' : '')
  switch (g.type) {
    case 'LineString':
      return line(g.coordinates, false)
    case 'MultiLineString':
      return g.coordinates.map((l) => line(l, false)).join('')
    case 'Polygon':
    case 'MultiPolygon':
      return polygonsOf(g)
        .flat()
        .map((r) => line(r, true))
        .join('')
    default:
      return ''
  }
}

/** 公園の色と斜線の向き。同じ地図の中の公園を見分ける(色だけに頼らず、向きも変える) */
export const PARK_STYLES = [
  { color: '#15803d', angle: 45 },
  { color: '#1d4ed8', angle: -45 },
  { color: '#9333ea', angle: 0 },
  { color: '#c2410c', angle: 90 },
] as const

/** 地図に出てくる公園の番号を、出てくる順に並べる(色と向きは、この順で割り当てる) */
export function parkCodesOf(map: { features: Feature[] }): string[] {
  const codes: string[] = []
  for (const f of map.features) {
    const p = f.properties
    const list =
      (p.kind === 'park' || p.kind === 'judge_park') && p.code
        ? [p.code]
        : p.kind === 'muni'
          ? (p.pota ?? []).map((q) => q.code)
          : []
    for (const c of list) if (!codes.includes(c)) codes.push(c)
  }
  return codes
}

export const parkStyleOf = (codes: string[], code: string) =>
  PARK_STYLES[Math.max(0, codes.indexOf(code)) % PARK_STYLES.length]

/**
 * 多角形の中で、地図の範囲 `bbox` に入る、文字を置く点。範囲の中で、多角形のいちばん広いところを横切る線の真ん中。
 * 範囲の中に多角形がなければ null
 */
export function labelPointIn(g: Geometry, bbox: BBox): Position | null {
  const [w, s, e, n] = bbox
  let best: { pos: Position; width: number } | null = null
  // 範囲を縦に 9 等分した横線で探す
  for (let k = 1; k <= 9; k++) {
    const y = s + ((n - s) * k) / 10
    const xs: number[] = []
    for (const ring of polygonsOf(g).flat()) {
      for (let i = 1; i < ring.length; i++) {
        const [x1, y1] = ring[i - 1]
        const [x2, y2] = ring[i]
        if (y1 > y !== y2 > y) xs.push(x1 + ((y - y1) * (x2 - x1)) / (y2 - y1))
      }
    }
    xs.sort((a, b) => a - b)
    for (let i = 0; i + 1 < xs.length; i += 2) {
      const a = Math.max(xs[i], w)
      const b = Math.min(xs[i + 1], e)
      if (b - a > (best?.width ?? 0)) best = { pos: [(a + b) / 2, y], width: b - a }
    }
  }
  return best?.pos ?? null
}
