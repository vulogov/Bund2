# RFC-0015 criterion 15's fixture for S12: alignment scores by Biopython
# 1.88's PairwiseAligner, which charges a gap of length L as
# open + (L - 1) * extend, EMBOSS's convention.
#
#     python3 -I align.py BLOSUM62 > align.txt
#
# BLOSUM62 is NCBI's blast/matrices/BLOSUM62, committed beside this script.
# Each line is: mode, matrix, gap.open, gap.extend, a, b, score. The test
# runs bio.align and bio.align.score on each and compares the score; the
# alignment it answers is checked by rescoring it, since equally good
# alignments may differ.
import sys

from Bio.Align import PairwiseAligner
from Bio.Align.substitution_matrices import Array, read

blosum62 = read(sys.argv[1])
dna = Array("ACGT", dims=2)
for x in "ACGT":
    for y in "ACGT":
        dna[x, y] = 5.0 if x == y else -4.0

PROTEIN = [
    ("HEAGAWGHEE", "PAWHEAE"),
    ("MKTAYIAKQRQISFVKSHFSRQ", "MKTAYIAKQISFVKSHFSRQ"),
    ("MEEPQSDPSVEPPLSQETFSDLWKLLPEN", "MEESQSDISLELPLSQETFSGLWKLLPPE"),
    ("WWWCCCWWW", "CCC"),
]
DNA = [
    ("GATTACA", "GCATGCT"),
    ("ACGTACGTTAGCATGCA", "ACGTTAGGCATGGCA"),
    ("TTTTACGTACGTAAAA", "ACGTACGT"),
    ("AGCTAGCTAGGCTTACG", "CTAGGATTACG"),
]
GAPS = [(10.0, 0.5), (5.0, 2.0)]


def score(mode, matrix, open_, extend, a, b):
    al = PairwiseAligner()
    al.substitution_matrix = matrix
    al.mode = "local" if mode == "local" else "global"
    al.open_gap_score = -open_
    al.extend_gap_score = -extend
    if mode == "semiglobal":
        al.open_end_gap_score = 0.0
        al.extend_end_gap_score = 0.0
    return al.score(a, b)


for name, matrix, pairs in [("blosum62", blosum62, PROTEIN), ("dna", dna, DNA)]:
    for a, b in pairs:
        for open_, extend in GAPS:
            for mode in ["global", "local", "semiglobal"]:
                s = score(mode, matrix, open_, extend, a, b)
                print(mode, name, open_, extend, a, b, repr(float(s)))
