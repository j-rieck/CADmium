import type Point3D from "./routes/threlte/Point3D.svelte"

declare module "nurbs"

export type WithTarget<Event, Target> = Event & { currentTarget: Target }

export type EntityType =
  "circle" |
  "arc" |
  "face" |
  "line" |
  "plane" |
  "point" |
  "point3D" |
  "meshFace"

export interface Entity {
  id: string
  type: EntityType
}

export interface Point {
  twoD: Vector2
  threeD: Vector3
  pointId: string
}

export interface Project {
  name: string
  assemblies: []
  workbenches: WorkBench[]
}

export interface WorkBench {
  name: string
  history: HistoryStep[]
  step_counters: {
    Extrusion: number, Plane: number, Point: number, Sketch: number
  }
}

export interface HistoryStep {
  name: string
  suppressed: boolean
  unique_id: string
  data: HistoryStepData
}

export type HistoryStepData =
  | PointData
  | PlaneData
  | ExtrusionData
  | SketchData

interface Point3 {
  x: number
  y: number
  z: number
}
interface Point3Hideable {
  x: number
  y: number
  z: number
  hidden: boolean
}

interface PointWithDelta {
  x: number
  y: number
  m: number
  dx: number
  dy: number
  fx: number
  fy: number
  fixed: boolean
  hidden: boolean
}

export interface SnapPoints {
  twoD: PointWithDelta
  threeD: Point3Hideable
  pointId: string // todo is number string - maybe change to number
}

interface PointData {
  type: "Point"
  point: Point3Hideable
}

interface PlaneData {
  type: "Plane"
  plane: {
    origin: Point3Hideable
    primary: Point3
    secondary: Point3
    tertiary: Point3
  },
  width: number  // %
  height: number // %
}

interface ExtrusionData {
  type: "Extrusion"
  extrusion: {
    sketch_id: string
    face_ids: number[]
    length: number
    offset: number
    direction: string // e.g "Normal"  todo enums
    mode: string // e,g "New"  todo enums
  }
}

interface IDictionary<TValue> {
  [id: string]: TValue
}

interface SegmentId {
  start: number
  end: number
}

interface Circle {
  center: number
  radius: number
  top: number
}

interface SketchData {
  type: "Sketch",
  plane_description: {
    PlaneId: string
  },
  width: number
  height: number
  sketch: {
    points: IDictionary<PointWithDelta>
    highest_point_id: number
    line_segments: IDictionary<SegmentId>
    highest_line_segment_id: number
    circles: IDictionary<Circle>
    highest_circle_id: number
    arcs: object // todo
    highest_arc_id: number
    constraints: object // todo
    highest_constraint_id: number
  }
}


export interface Realization {
  planes: object
  points: object
  sketches: object
  solids: object
}

export interface ExtrusionSketchData {
  sketch_id: string
  face_ids: string[]
  length: string // todo change to number
  offset: number
  direction: string
  mode: string
}

// rust expects:
// `RenameWorkbench`
// `RenameStep`
// `RenameProject`
// `DeleteLines`
// `DeleteArcs`
// `DeleteCircles`
// `NewPointOnSketch`
// `NewPointOnSketch2`
// `NewCircleBetweenPoints`
// `NewRectangleBetweenPoints`
// `NewLineOnSketch`
// `DeleteLineSegment`
// `StepSketch`
// `SolveSketch`
// `NewSketchOnPlane`
// `SetSketchPlane`
// `DeleteSketch`
// `NewExtrusion`
// `UpdateExtrusion`
// `UpdateExtrusionLength`

// todo these interfaces should be exported from rust with wasm bindgen ?

// io messages sent to rust
interface UpdateExtrusion {
  workbench_id: number
  sketch_id: string
  face_ids: number[]
  length: number
  offset: 0.0
  extrusion_name: "Extra"
  direction: "Normal"
  extrusion_id: string
}

interface SetSketchPlane {
  workbench_id: number
  sketch_id: string
  plane_id: string
}

interface NewSketchOnPlane {
  workbench_id: number
  plane_id: string
  sketch_name: string
}

interface NewExtrusion {
  workbench_id: number
  sketch_id: string
  face_ids: number[]
  length: number
  offset: number
  extrusion_name: string
  direction: string
}

interface DeleteLines {
  workbench_id: number
  sketch_id: string
  line_ids: number[]
}

interface DeleteArcs {
  workbench_id: number
  sketch_id: string
  arc_ids: number[]
}

interface DeleteCircles {
  workbench_id: number
  sketch_id: string
  circle_ids: number[]
}

interface NewRectangleBetweenPoints {
  workbench_id: number
  sketch_id: string
  start_id: number
  end_id: number
}

interface NewCircleBetweenPoints {
  workbench_id: number
  sketch_id: string
  center_id: number
  edge_id: number
}

interface NewLineOnSketch {
  workbench_id: number
  sketch_id: string
  start_point_id: number
  end_point_id: number
}

interface NewPointOnSketch2 {
  workbench_id: number
  sketch_id: string
  x: Vector2["x"]
  y: Vector2["y"]
  hidden: boolean
}

interface RenameStep {
  workbench_id: number
  step_id: number
  new_name: string
}

export type Message =
  | { UpdateExtrusion: UpdateExtrusion }
  | { SetSketchPlane: SetSketchPlane }
  | { NewSketchOnPlane: NewSketchOnPlane }
  | { NewExtrusion: NewExtrusion }
  | { DeleteLines: DeleteLines }
  | { DeleteArcs: DeleteArcs }
  | { DeleteCircles: DeleteCircles }
  | { NewRectangleBetweenPoints: NewRectangleBetweenPoints }
  | { NewCircleBetweenPoints: NewCircleBetweenPoints }
  | { NewLineOnSketch: NewLineOnSketch }
  | { NewPointOnSketch2: NewPointOnSketch2 }
  | { RenameStep: RenameStep }

export interface MessageHistory {
  message: Message
  result: any
}
