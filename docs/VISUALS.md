# Cross-reference visuals

`sbr-viz` renders cross-reference data as standalone SVG (vector, hover titles on
every mark, editable in Inkscape/Illustrator) or PNG (rasterized with `resvg`).

```sh
curl -LO https://a.openbible.info/data/cross-references.zip   # OpenBible.info, CC BY
sbr viz pair   --xrefs cross-references.zip --left Ex --right Jas -o ex-jas.svg
sbr viz matrix --xrefs cross-references.zip --theme dark -o matrix.png --scale 2
sbr viz arc    --xrefs cross-references.zip -o arc.png
sbr viz chord  --xrefs cross-references.zip -o chord.svg
sbr viz gallery --xrefs cross-references.zip --out-dir visuals/   # everything, both themes
```

| chart | shows |
|---|---|
| `pair` | two books on parallel axes; chapter bands sized by verse count, chapter titles (`ChapterHeadingUtil`), one curve per cross-reference, weight by votes |
| `matrix` | 66 × 66 books; each cell split into `--detail` × `--detail` bins by position in both books, coloured by testament |
| `arc` | all 1,189 chapters on one line (bars = verses), strongest chapter pairs as arcs coloured by distance |
| `chord` | books on a circle grouped by genre, ribbons for the strongest book pairs |

Common options: `--theme light|dark`, `--min-votes N` (default 1, drops
down-voted links), `--scale S` for PNG resolution. Output format follows the
file extension. Books accept abbreviations, OSIS ids, English names or numbers.
Input is the OpenBible layout (`From Verse<TAB>To Verse[-To Verse]<TAB>Votes`,
OSIS ids), as `.txt` or the original `.zip`.
