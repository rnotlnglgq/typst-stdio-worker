#import "@preview/cetz:0.5.2"
#import "@preview/fletcher:0.5.8": diagram, node, edge
#import "@preview/physica:0.9.8": *
#import "@preview/outrageous:0.4.1"
#import "@preview/showybox:2.0.4": showybox


= Typst Prelude 功能演示

// 1. Outline 展示 (由 outrageous 驱动)
#outline()

== 1. 物理与数学 (Physica)
借助 `physica`，我们可以轻松排版狄拉克符号：
$ bra(phi) ket(psi) = frac(1, sqrt(2)) $
以及复杂的矩阵算子：
$ grad product v = det(mat(1_x, 1_y; partial_x, partial_y; v_x, v_y)) $

== 2. 逻辑图表 (Fletcher)
使用 `fletcher` 绘制一个简单的状态机：
#align(center)[
  #diagram(
    node((0,0), [Start], radius: 2em),
    edge("-|>"),
    node((1,0), [Process], stroke: 1pt),
    edge("-|>"),
    node((2,0), [End], radius: 2em, stroke: 2pt),
  )
]

== 3. 矢量绘图 (CeTZ)
使用 `cetz` 绘制几何图形：
#cetz.canvas({
  import cetz.draw: *
  circle((0,0), radius: 1, fill: blue.lighten(80%))
  line((-1,0), (1,0), stroke: red)
  content((0, 1.2), [Unit Circle])
})