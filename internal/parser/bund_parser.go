// Code generated from Bund.g4 by ANTLR 4.9.1. DO NOT EDIT.

package parser // Bund

import (
	"fmt"
	"reflect"
	"strconv"

	"github.com/antlr/antlr4/runtime/Go/antlr"
)

// Suppress unused import errors
var _ = fmt.Printf
var _ = reflect.Copy
var _ = strconv.Itoa

var parserATN = []uint16{
	3, 24715, 42794, 33075, 47597, 16764, 15335, 30598, 22884, 3, 45, 267,
	4, 2, 9, 2, 4, 3, 9, 3, 4, 4, 9, 4, 4, 5, 9, 5, 4, 6, 9, 6, 4, 7, 9, 7,
	4, 8, 9, 8, 4, 9, 9, 9, 4, 10, 9, 10, 4, 11, 9, 11, 4, 12, 9, 12, 4, 13,
	9, 13, 4, 14, 9, 14, 4, 15, 9, 15, 4, 16, 9, 16, 4, 17, 9, 17, 4, 18, 9,
	18, 4, 19, 9, 19, 4, 20, 9, 20, 4, 21, 9, 21, 4, 22, 9, 22, 4, 23, 9, 23,
	4, 24, 9, 24, 3, 2, 7, 2, 50, 10, 2, 12, 2, 14, 2, 53, 11, 2, 3, 3, 3,
	3, 5, 3, 57, 10, 3, 3, 4, 3, 4, 3, 4, 3, 4, 7, 4, 63, 10, 4, 12, 4, 14,
	4, 66, 11, 4, 3, 4, 3, 4, 3, 5, 3, 5, 7, 5, 72, 10, 5, 12, 5, 14, 5, 75,
	11, 5, 3, 5, 3, 5, 3, 5, 3, 5, 5, 5, 81, 10, 5, 3, 6, 3, 6, 3, 6, 3, 6,
	3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6, 3, 6,
	3, 6, 3, 6, 5, 6, 101, 10, 6, 3, 7, 3, 7, 3, 7, 3, 7, 3, 7, 3, 7, 3, 7,
	3, 7, 3, 7, 5, 7, 112, 10, 7, 3, 8, 3, 8, 5, 8, 116, 10, 8, 3, 8, 3, 8,
	3, 8, 3, 8, 5, 8, 122, 10, 8, 3, 9, 3, 9, 3, 9, 5, 9, 127, 10, 9, 3, 9,
	3, 9, 3, 9, 3, 9, 5, 9, 133, 10, 9, 3, 10, 3, 10, 5, 10, 137, 10, 10, 3,
	10, 3, 10, 3, 10, 3, 10, 5, 10, 143, 10, 10, 3, 11, 3, 11, 5, 11, 147,
	10, 11, 3, 11, 3, 11, 3, 11, 3, 11, 5, 11, 153, 10, 11, 3, 12, 3, 12, 5,
	12, 157, 10, 12, 3, 12, 3, 12, 3, 12, 3, 12, 5, 12, 163, 10, 12, 3, 13,
	3, 13, 5, 13, 167, 10, 13, 3, 13, 3, 13, 3, 13, 3, 13, 5, 13, 173, 10,
	13, 3, 14, 3, 14, 5, 14, 177, 10, 14, 3, 14, 3, 14, 3, 14, 3, 14, 5, 14,
	183, 10, 14, 3, 15, 3, 15, 3, 16, 3, 16, 3, 17, 3, 17, 7, 17, 191, 10,
	17, 12, 17, 14, 17, 194, 11, 17, 3, 17, 3, 17, 3, 17, 3, 17, 5, 17, 200,
	10, 17, 3, 18, 3, 18, 6, 18, 204, 10, 18, 13, 18, 14, 18, 205, 3, 18, 3,
	18, 3, 19, 3, 19, 7, 19, 212, 10, 19, 12, 19, 14, 19, 215, 11, 19, 3, 19,
	3, 19, 3, 20, 3, 20, 3, 20, 3, 20, 7, 20, 223, 10, 20, 12, 20, 14, 20,
	226, 11, 20, 3, 20, 3, 20, 3, 21, 3, 21, 7, 21, 232, 10, 21, 12, 21, 14,
	21, 235, 11, 21, 3, 21, 3, 21, 3, 22, 3, 22, 3, 22, 3, 22, 7, 22, 243,
	10, 22, 12, 22, 14, 22, 246, 11, 22, 3, 22, 3, 22, 3, 23, 3, 23, 3, 23,
	3, 23, 7, 23, 254, 10, 23, 12, 23, 14, 23, 257, 11, 23, 3, 23, 3, 23, 3,
	24, 3, 24, 3, 24, 3, 24, 5, 24, 265, 10, 24, 3, 24, 2, 2, 25, 2, 4, 6,
	8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40, 42,
	44, 46, 2, 8, 5, 2, 34, 34, 36, 36, 38, 38, 4, 2, 34, 36, 38, 38, 3, 2,
	32, 33, 4, 2, 10, 13, 29, 29, 3, 2, 39, 40, 3, 2, 16, 17, 2, 296, 2, 51,
	3, 2, 2, 2, 4, 56, 3, 2, 2, 2, 6, 58, 3, 2, 2, 2, 8, 69, 3, 2, 2, 2, 10,
	100, 3, 2, 2, 2, 12, 111, 3, 2, 2, 2, 14, 115, 3, 2, 2, 2, 16, 123, 3,
	2, 2, 2, 18, 136, 3, 2, 2, 2, 20, 146, 3, 2, 2, 2, 22, 156, 3, 2, 2, 2,
	24, 166, 3, 2, 2, 2, 26, 176, 3, 2, 2, 2, 28, 184, 3, 2, 2, 2, 30, 186,
	3, 2, 2, 2, 32, 188, 3, 2, 2, 2, 34, 201, 3, 2, 2, 2, 36, 209, 3, 2, 2,
	2, 38, 218, 3, 2, 2, 2, 40, 229, 3, 2, 2, 2, 42, 238, 3, 2, 2, 2, 44, 249,
	3, 2, 2, 2, 46, 260, 3, 2, 2, 2, 48, 50, 5, 4, 3, 2, 49, 48, 3, 2, 2, 2,
	50, 53, 3, 2, 2, 2, 51, 49, 3, 2, 2, 2, 51, 52, 3, 2, 2, 2, 52, 3, 3, 2,
	2, 2, 53, 51, 3, 2, 2, 2, 54, 57, 5, 6, 4, 2, 55, 57, 5, 8, 5, 2, 56, 54,
	3, 2, 2, 2, 56, 55, 3, 2, 2, 2, 57, 5, 3, 2, 2, 2, 58, 59, 7, 3, 2, 2,
	59, 60, 7, 38, 2, 2, 60, 64, 7, 39, 2, 2, 61, 63, 5, 10, 6, 2, 62, 61,
	3, 2, 2, 2, 63, 66, 3, 2, 2, 2, 64, 62, 3, 2, 2, 2, 64, 65, 3, 2, 2, 2,
	65, 67, 3, 2, 2, 2, 66, 64, 3, 2, 2, 2, 67, 68, 7, 4, 2, 2, 68, 7, 3, 2,
	2, 2, 69, 73, 7, 5, 2, 2, 70, 72, 5, 10, 6, 2, 71, 70, 3, 2, 2, 2, 72,
	75, 3, 2, 2, 2, 73, 71, 3, 2, 2, 2, 73, 74, 3, 2, 2, 2, 74, 76, 3, 2, 2,
	2, 75, 73, 3, 2, 2, 2, 76, 80, 7, 6, 2, 2, 77, 78, 7, 7, 2, 2, 78, 79,
	9, 2, 2, 2, 79, 81, 7, 6, 2, 2, 80, 77, 3, 2, 2, 2, 80, 81, 3, 2, 2, 2,
	81, 9, 3, 2, 2, 2, 82, 101, 5, 6, 4, 2, 83, 101, 5, 8, 5, 2, 84, 101, 5,
	14, 8, 2, 85, 101, 5, 16, 9, 2, 86, 101, 5, 18, 10, 2, 87, 101, 5, 20,
	11, 2, 88, 101, 5, 22, 12, 2, 89, 101, 5, 24, 13, 2, 90, 101, 5, 26, 14,
	2, 91, 101, 5, 32, 17, 2, 92, 101, 5, 34, 18, 2, 93, 101, 5, 36, 19, 2,
	94, 101, 5, 28, 15, 2, 95, 101, 5, 30, 16, 2, 96, 101, 5, 38, 20, 2, 97,
	101, 5, 40, 21, 2, 98, 101, 5, 44, 23, 2, 99, 101, 5, 46, 24, 2, 100, 82,
	3, 2, 2, 2, 100, 83, 3, 2, 2, 2, 100, 84, 3, 2, 2, 2, 100, 85, 3, 2, 2,
	2, 100, 86, 3, 2, 2, 2, 100, 87, 3, 2, 2, 2, 100, 88, 3, 2, 2, 2, 100,
	89, 3, 2, 2, 2, 100, 90, 3, 2, 2, 2, 100, 91, 3, 2, 2, 2, 100, 92, 3, 2,
	2, 2, 100, 93, 3, 2, 2, 2, 100, 94, 3, 2, 2, 2, 100, 95, 3, 2, 2, 2, 100,
	96, 3, 2, 2, 2, 100, 97, 3, 2, 2, 2, 100, 98, 3, 2, 2, 2, 100, 99, 3, 2,
	2, 2, 101, 11, 3, 2, 2, 2, 102, 112, 5, 18, 10, 2, 103, 112, 5, 20, 11,
	2, 104, 112, 5, 22, 12, 2, 105, 112, 5, 24, 13, 2, 106, 112, 5, 26, 14,
	2, 107, 112, 5, 14, 8, 2, 108, 112, 5, 30, 16, 2, 109, 112, 5, 40, 21,
	2, 110, 112, 5, 16, 9, 2, 111, 102, 3, 2, 2, 2, 111, 103, 3, 2, 2, 2, 111,
	104, 3, 2, 2, 2, 111, 105, 3, 2, 2, 2, 111, 106, 3, 2, 2, 2, 111, 107,
	3, 2, 2, 2, 111, 108, 3, 2, 2, 2, 111, 109, 3, 2, 2, 2, 111, 110, 3, 2,
	2, 2, 112, 13, 3, 2, 2, 2, 113, 114, 7, 38, 2, 2, 114, 116, 7, 8, 2, 2,
	115, 113, 3, 2, 2, 2, 115, 116, 3, 2, 2, 2, 116, 117, 3, 2, 2, 2, 117,
	121, 9, 3, 2, 2, 118, 119, 7, 7, 2, 2, 119, 120, 9, 3, 2, 2, 120, 122,
	7, 6, 2, 2, 121, 118, 3, 2, 2, 2, 121, 122, 3, 2, 2, 2, 122, 15, 3, 2,
	2, 2, 123, 126, 7, 9, 2, 2, 124, 125, 7, 38, 2, 2, 125, 127, 7, 8, 2, 2,
	126, 124, 3, 2, 2, 2, 126, 127, 3, 2, 2, 2, 127, 128, 3, 2, 2, 2, 128,
	132, 9, 3, 2, 2, 129, 130, 7, 7, 2, 2, 130, 131, 9, 3, 2, 2, 131, 133,
	7, 6, 2, 2, 132, 129, 3, 2, 2, 2, 132, 133, 3, 2, 2, 2, 133, 17, 3, 2,
	2, 2, 134, 135, 7, 38, 2, 2, 135, 137, 7, 8, 2, 2, 136, 134, 3, 2, 2, 2,
	136, 137, 3, 2, 2, 2, 137, 138, 3, 2, 2, 2, 138, 142, 9, 4, 2, 2, 139,
	140, 7, 7, 2, 2, 140, 141, 9, 3, 2, 2, 141, 143, 7, 6, 2, 2, 142, 139,
	3, 2, 2, 2, 142, 143, 3, 2, 2, 2, 143, 19, 3, 2, 2, 2, 144, 145, 7, 38,
	2, 2, 145, 147, 7, 8, 2, 2, 146, 144, 3, 2, 2, 2, 146, 147, 3, 2, 2, 2,
	147, 148, 3, 2, 2, 2, 148, 152, 7, 27, 2, 2, 149, 150, 7, 7, 2, 2, 150,
	151, 9, 3, 2, 2, 151, 153, 7, 6, 2, 2, 152, 149, 3, 2, 2, 2, 152, 153,
	3, 2, 2, 2, 153, 21, 3, 2, 2, 2, 154, 155, 7, 38, 2, 2, 155, 157, 7, 8,
	2, 2, 156, 154, 3, 2, 2, 2, 156, 157, 3, 2, 2, 2, 157, 158, 3, 2, 2, 2,
	158, 162, 9, 5, 2, 2, 159, 160, 7, 7, 2, 2, 160, 161, 9, 3, 2, 2, 161,
	163, 7, 6, 2, 2, 162, 159, 3, 2, 2, 2, 162, 163, 3, 2, 2, 2, 163, 23, 3,
	2, 2, 2, 164, 165, 7, 38, 2, 2, 165, 167, 7, 8, 2, 2, 166, 164, 3, 2, 2,
	2, 166, 167, 3, 2, 2, 2, 167, 168, 3, 2, 2, 2, 168, 172, 7, 30, 2, 2, 169,
	170, 7, 7, 2, 2, 170, 171, 9, 3, 2, 2, 171, 173, 7, 6, 2, 2, 172, 169,
	3, 2, 2, 2, 172, 173, 3, 2, 2, 2, 173, 25, 3, 2, 2, 2, 174, 175, 7, 38,
	2, 2, 175, 177, 7, 8, 2, 2, 176, 174, 3, 2, 2, 2, 176, 177, 3, 2, 2, 2,
	177, 178, 3, 2, 2, 2, 178, 182, 7, 31, 2, 2, 179, 180, 7, 7, 2, 2, 180,
	181, 9, 3, 2, 2, 181, 183, 7, 6, 2, 2, 182, 179, 3, 2, 2, 2, 182, 183,
	3, 2, 2, 2, 183, 27, 3, 2, 2, 2, 184, 185, 9, 6, 2, 2, 185, 29, 3, 2, 2,
	2, 186, 187, 7, 41, 2, 2, 187, 31, 3, 2, 2, 2, 188, 192, 7, 14, 2, 2, 189,
	191, 5, 12, 7, 2, 190, 189, 3, 2, 2, 2, 191, 194, 3, 2, 2, 2, 192, 190,
	3, 2, 2, 2, 192, 193, 3, 2, 2, 2, 193, 195, 3, 2, 2, 2, 194, 192, 3, 2,
	2, 2, 195, 199, 7, 6, 2, 2, 196, 197, 7, 7, 2, 2, 197, 198, 9, 3, 2, 2,
	198, 200, 7, 6, 2, 2, 199, 196, 3, 2, 2, 2, 199, 200, 3, 2, 2, 2, 200,
	33, 3, 2, 2, 2, 201, 203, 7, 15, 2, 2, 202, 204, 5, 12, 7, 2, 203, 202,
	3, 2, 2, 2, 204, 205, 3, 2, 2, 2, 205, 203, 3, 2, 2, 2, 205, 206, 3, 2,
	2, 2, 206, 207, 3, 2, 2, 2, 207, 208, 7, 6, 2, 2, 208, 35, 3, 2, 2, 2,
	209, 213, 9, 7, 2, 2, 210, 212, 5, 10, 6, 2, 211, 210, 3, 2, 2, 2, 212,
	215, 3, 2, 2, 2, 213, 211, 3, 2, 2, 2, 213, 214, 3, 2, 2, 2, 214, 216,
	3, 2, 2, 2, 215, 213, 3, 2, 2, 2, 216, 217, 7, 6, 2, 2, 217, 37, 3, 2,
	2, 2, 218, 219, 7, 3, 2, 2, 219, 220, 7, 38, 2, 2, 220, 224, 7, 18, 2,
	2, 221, 223, 5, 10, 6, 2, 222, 221, 3, 2, 2, 2, 223, 226, 3, 2, 2, 2, 224,
	222, 3, 2, 2, 2, 224, 225, 3, 2, 2, 2, 225, 227, 3, 2, 2, 2, 226, 224,
	3, 2, 2, 2, 227, 228, 7, 19, 2, 2, 228, 39, 3, 2, 2, 2, 229, 233, 7, 20,
	2, 2, 230, 232, 5, 10, 6, 2, 231, 230, 3, 2, 2, 2, 232, 235, 3, 2, 2, 2,
	233, 231, 3, 2, 2, 2, 233, 234, 3, 2, 2, 2, 234, 236, 3, 2, 2, 2, 235,
	233, 3, 2, 2, 2, 236, 237, 7, 19, 2, 2, 237, 41, 3, 2, 2, 2, 238, 239,
	7, 21, 2, 2, 239, 240, 7, 35, 2, 2, 240, 244, 7, 22, 2, 2, 241, 243, 5,
	10, 6, 2, 242, 241, 3, 2, 2, 2, 243, 246, 3, 2, 2, 2, 244, 242, 3, 2, 2,
	2, 244, 245, 3, 2, 2, 2, 245, 247, 3, 2, 2, 2, 246, 244, 3, 2, 2, 2, 247,
	248, 7, 19, 2, 2, 248, 43, 3, 2, 2, 2, 249, 250, 7, 23, 2, 2, 250, 251,
	7, 35, 2, 2, 251, 255, 7, 24, 2, 2, 252, 254, 5, 10, 6, 2, 253, 252, 3,
	2, 2, 2, 254, 257, 3, 2, 2, 2, 255, 253, 3, 2, 2, 2, 255, 256, 3, 2, 2,
	2, 256, 258, 3, 2, 2, 2, 257, 255, 3, 2, 2, 2, 258, 259, 7, 19, 2, 2, 259,
	45, 3, 2, 2, 2, 260, 261, 7, 25, 2, 2, 261, 264, 7, 27, 2, 2, 262, 263,
	7, 26, 2, 2, 263, 265, 7, 27, 2, 2, 264, 262, 3, 2, 2, 2, 264, 265, 3,
	2, 2, 2, 265, 47, 3, 2, 2, 2, 32, 51, 56, 64, 73, 80, 100, 111, 115, 121,
	126, 132, 136, 142, 146, 152, 156, 162, 166, 172, 176, 182, 192, 199, 205,
	213, 224, 233, 244, 255, 264,
}
var literalNames = []string{
	"", "'['", "';;'", "'('", "')'", "':('", "'@'", "'`'", "'+Inf'", "'NaN'",
	"'-Inf'", "'Inf'", "'(*'", "'(?'", "'(true'", "'(false'", "']'", "'.'",
	"'[]'", "'[['", "']]'", "'[[['", "']]]'", "'#'", "'::'", "", "", "", "",
	"", "", "", "", "", "", "'/'", "", "':'", "';'", "'|'",
}
var symbolicNames = []string{
	"", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
	"", "", "", "", "", "", "", "INTEGER", "DECIMAL_INTEGER", "FLOAT_NUMBER",
	"STRING", "COMPLEX_NUMBER", "TRUE", "FALSE", "SYS", "CMD", "SYSF", "SLASH",
	"NAME", "TOBEGIN", "TOEND", "SEPARATE", "COMMENT", "BLOCK_COMMENT", "WS",
	"SHEBANG",
}

var ruleNames = []string{
	"expressions", "root_term", "ns", "block", "term", "data", "call_term",
	"ref_call_term", "boolean_term", "integer_term", "float_term", "string_term",
	"complex_term", "mode_term", "separate_term", "datablock_term", "matchblock_term",
	"logicblock_term", "function_term", "lambda_term", "operation_term", "generator_term",
	"index_term",
}

type BundParser struct {
	*antlr.BaseParser
}

// NewBundParser produces a new parser instance for the optional input antlr.TokenStream.
//
// The *BundParser instance produced may be reused by calling the SetInputStream method.
// The initial parser configuration is expensive to construct, and the object is not thread-safe;
// however, if used within a Golang sync.Pool, the construction cost amortizes well and the
// objects can be used in a thread-safe manner.
func NewBundParser(input antlr.TokenStream) *BundParser {
	this := new(BundParser)
	deserializer := antlr.NewATNDeserializer(nil)
	deserializedATN := deserializer.DeserializeFromUInt16(parserATN)
	decisionToDFA := make([]*antlr.DFA, len(deserializedATN.DecisionToState))
	for index, ds := range deserializedATN.DecisionToState {
		decisionToDFA[index] = antlr.NewDFA(ds, index)
	}
	this.BaseParser = antlr.NewBaseParser(input)

	this.Interpreter = antlr.NewParserATNSimulator(this, deserializedATN, decisionToDFA, antlr.NewPredictionContextCache())
	this.RuleNames = ruleNames
	this.LiteralNames = literalNames
	this.SymbolicNames = symbolicNames
	this.GrammarFileName = "Bund.g4"

	return this
}

// BundParser tokens.
const (
	BundParserEOF             = antlr.TokenEOF
	BundParserT__0            = 1
	BundParserT__1            = 2
	BundParserT__2            = 3
	BundParserT__3            = 4
	BundParserT__4            = 5
	BundParserT__5            = 6
	BundParserT__6            = 7
	BundParserT__7            = 8
	BundParserT__8            = 9
	BundParserT__9            = 10
	BundParserT__10           = 11
	BundParserT__11           = 12
	BundParserT__12           = 13
	BundParserT__13           = 14
	BundParserT__14           = 15
	BundParserT__15           = 16
	BundParserT__16           = 17
	BundParserT__17           = 18
	BundParserT__18           = 19
	BundParserT__19           = 20
	BundParserT__20           = 21
	BundParserT__21           = 22
	BundParserT__22           = 23
	BundParserT__23           = 24
	BundParserINTEGER         = 25
	BundParserDECIMAL_INTEGER = 26
	BundParserFLOAT_NUMBER    = 27
	BundParserSTRING          = 28
	BundParserCOMPLEX_NUMBER  = 29
	BundParserTRUE            = 30
	BundParserFALSE           = 31
	BundParserSYS             = 32
	BundParserCMD             = 33
	BundParserSYSF            = 34
	BundParserSLASH           = 35
	BundParserNAME            = 36
	BundParserTOBEGIN         = 37
	BundParserTOEND           = 38
	BundParserSEPARATE        = 39
	BundParserCOMMENT         = 40
	BundParserBLOCK_COMMENT   = 41
	BundParserWS              = 42
	BundParserSHEBANG         = 43
)

// BundParser rules.
const (
	BundParserRULE_expressions     = 0
	BundParserRULE_root_term       = 1
	BundParserRULE_ns              = 2
	BundParserRULE_block           = 3
	BundParserRULE_term            = 4
	BundParserRULE_data            = 5
	BundParserRULE_call_term       = 6
	BundParserRULE_ref_call_term   = 7
	BundParserRULE_boolean_term    = 8
	BundParserRULE_integer_term    = 9
	BundParserRULE_float_term      = 10
	BundParserRULE_string_term     = 11
	BundParserRULE_complex_term    = 12
	BundParserRULE_mode_term       = 13
	BundParserRULE_separate_term   = 14
	BundParserRULE_datablock_term  = 15
	BundParserRULE_matchblock_term = 16
	BundParserRULE_logicblock_term = 17
	BundParserRULE_function_term   = 18
	BundParserRULE_lambda_term     = 19
	BundParserRULE_operation_term  = 20
	BundParserRULE_generator_term  = 21
	BundParserRULE_index_term      = 22
)

// IExpressionsContext is an interface to support dynamic dispatch.
type IExpressionsContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// IsExpressionsContext differentiates from other interfaces.
	IsExpressionsContext()
}

type ExpressionsContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
}

func NewEmptyExpressionsContext() *ExpressionsContext {
	var p = new(ExpressionsContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_expressions
	return p
}

func (*ExpressionsContext) IsExpressionsContext() {}

func NewExpressionsContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *ExpressionsContext {
	var p = new(ExpressionsContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_expressions

	return p
}

func (s *ExpressionsContext) GetParser() antlr.Parser { return s.parser }

func (s *ExpressionsContext) AllRoot_term() []IRoot_termContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*IRoot_termContext)(nil)).Elem())
	var tst = make([]IRoot_termContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(IRoot_termContext)
		}
	}

	return tst
}

func (s *ExpressionsContext) Root_term(i int) IRoot_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IRoot_termContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(IRoot_termContext)
}

func (s *ExpressionsContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *ExpressionsContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *ExpressionsContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterExpressions(s)
	}
}

func (s *ExpressionsContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitExpressions(s)
	}
}

func (p *BundParser) Expressions() (localctx IExpressionsContext) {
	localctx = NewExpressionsContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 0, BundParserRULE_expressions)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(49)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for _la == BundParserT__0 || _la == BundParserT__2 {
		{
			p.SetState(46)
			p.Root_term()
		}

		p.SetState(51)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}

	return localctx
}

// IRoot_termContext is an interface to support dynamic dispatch.
type IRoot_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// IsRoot_termContext differentiates from other interfaces.
	IsRoot_termContext()
}

type Root_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
}

func NewEmptyRoot_termContext() *Root_termContext {
	var p = new(Root_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_root_term
	return p
}

func (*Root_termContext) IsRoot_termContext() {}

func NewRoot_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Root_termContext {
	var p = new(Root_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_root_term

	return p
}

func (s *Root_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Root_termContext) Ns() INsContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*INsContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(INsContext)
}

func (s *Root_termContext) Block() IBlockContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IBlockContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IBlockContext)
}

func (s *Root_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Root_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Root_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterRoot_term(s)
	}
}

func (s *Root_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitRoot_term(s)
	}
}

func (p *BundParser) Root_term() (localctx IRoot_termContext) {
	localctx = NewRoot_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 2, BundParserRULE_root_term)

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(54)
	p.GetErrorHandler().Sync(p)

	switch p.GetTokenStream().LA(1) {
	case BundParserT__0:
		{
			p.SetState(52)
			p.Ns()
		}

	case BundParserT__2:
		{
			p.SetState(53)
			p.Block()
		}

	default:
		panic(antlr.NewNoViableAltException(p, nil, nil, nil, nil, nil))
	}

	return localctx
}

// INsContext is an interface to support dynamic dispatch.
type INsContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetName returns the name token.
	GetName() antlr.Token

	// SetName sets the name token.
	SetName(antlr.Token)

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsNsContext differentiates from other interfaces.
	IsNsContext()
}

type NsContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	name   antlr.Token
	_term  ITermContext
	body   []ITermContext
}

func NewEmptyNsContext() *NsContext {
	var p = new(NsContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_ns
	return p
}

func (*NsContext) IsNsContext() {}

func NewNsContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *NsContext {
	var p = new(NsContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_ns

	return p
}

func (s *NsContext) GetParser() antlr.Parser { return s.parser }

func (s *NsContext) GetName() antlr.Token { return s.name }

func (s *NsContext) SetName(v antlr.Token) { s.name = v }

func (s *NsContext) Get_term() ITermContext { return s._term }

func (s *NsContext) Set_term(v ITermContext) { s._term = v }

func (s *NsContext) GetBody() []ITermContext { return s.body }

func (s *NsContext) SetBody(v []ITermContext) { s.body = v }

func (s *NsContext) TOBEGIN() antlr.TerminalNode {
	return s.GetToken(BundParserTOBEGIN, 0)
}

func (s *NsContext) NAME() antlr.TerminalNode {
	return s.GetToken(BundParserNAME, 0)
}

func (s *NsContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *NsContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *NsContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *NsContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *NsContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterNs(s)
	}
}

func (s *NsContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitNs(s)
	}
}

func (p *BundParser) Ns() (localctx INsContext) {
	localctx = NewNsContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 4, BundParserRULE_ns)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(56)
		p.Match(BundParserT__0)
	}
	{
		p.SetState(57)

		var _m = p.Match(BundParserNAME)

		localctx.(*NsContext).name = _m
	}
	{
		p.SetState(58)
		p.Match(BundParserTOBEGIN)
	}
	p.SetState(62)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(59)

			var _x = p.Term()

			localctx.(*NsContext)._term = _x
		}
		localctx.(*NsContext).body = append(localctx.(*NsContext).body, localctx.(*NsContext)._term)

		p.SetState(64)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(65)
		p.Match(BundParserT__1)
	}

	return localctx
}

// IBlockContext is an interface to support dynamic dispatch.
type IBlockContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsBlockContext differentiates from other interfaces.
	IsBlockContext()
}

type BlockContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	_term   ITermContext
	body    []ITermContext
	FUNCTOR antlr.Token
}

func NewEmptyBlockContext() *BlockContext {
	var p = new(BlockContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_block
	return p
}

func (*BlockContext) IsBlockContext() {}

func NewBlockContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *BlockContext {
	var p = new(BlockContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_block

	return p
}

func (s *BlockContext) GetParser() antlr.Parser { return s.parser }

func (s *BlockContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *BlockContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *BlockContext) Get_term() ITermContext { return s._term }

func (s *BlockContext) Set_term(v ITermContext) { s._term = v }

func (s *BlockContext) GetBody() []ITermContext { return s.body }

func (s *BlockContext) SetBody(v []ITermContext) { s.body = v }

func (s *BlockContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *BlockContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *BlockContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *BlockContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *BlockContext) NAME() antlr.TerminalNode {
	return s.GetToken(BundParserNAME, 0)
}

func (s *BlockContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *BlockContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *BlockContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterBlock(s)
	}
}

func (s *BlockContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitBlock(s)
	}
}

func (p *BundParser) Block() (localctx IBlockContext) {
	localctx = NewBlockContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 6, BundParserRULE_block)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(67)
		p.Match(BundParserT__2)
	}
	p.SetState(71)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(68)

			var _x = p.Term()

			localctx.(*BlockContext)._term = _x
		}
		localctx.(*BlockContext).body = append(localctx.(*BlockContext).body, localctx.(*BlockContext)._term)

		p.SetState(73)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(74)
		p.Match(BundParserT__3)
	}
	p.SetState(78)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(75)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(76)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*BlockContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*BlockContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(77)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// ITermContext is an interface to support dynamic dispatch.
type ITermContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// IsTermContext differentiates from other interfaces.
	IsTermContext()
}

type TermContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
}

func NewEmptyTermContext() *TermContext {
	var p = new(TermContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_term
	return p
}

func (*TermContext) IsTermContext() {}

func NewTermContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *TermContext {
	var p = new(TermContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_term

	return p
}

func (s *TermContext) GetParser() antlr.Parser { return s.parser }

func (s *TermContext) Ns() INsContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*INsContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(INsContext)
}

func (s *TermContext) Block() IBlockContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IBlockContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IBlockContext)
}

func (s *TermContext) Call_term() ICall_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ICall_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ICall_termContext)
}

func (s *TermContext) Ref_call_term() IRef_call_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IRef_call_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IRef_call_termContext)
}

func (s *TermContext) Boolean_term() IBoolean_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IBoolean_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IBoolean_termContext)
}

func (s *TermContext) Integer_term() IInteger_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IInteger_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IInteger_termContext)
}

func (s *TermContext) Float_term() IFloat_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IFloat_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IFloat_termContext)
}

func (s *TermContext) String_term() IString_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IString_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IString_termContext)
}

func (s *TermContext) Complex_term() IComplex_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IComplex_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IComplex_termContext)
}

func (s *TermContext) Datablock_term() IDatablock_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IDatablock_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IDatablock_termContext)
}

func (s *TermContext) Matchblock_term() IMatchblock_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IMatchblock_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IMatchblock_termContext)
}

func (s *TermContext) Logicblock_term() ILogicblock_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ILogicblock_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ILogicblock_termContext)
}

func (s *TermContext) Mode_term() IMode_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IMode_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IMode_termContext)
}

func (s *TermContext) Separate_term() ISeparate_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ISeparate_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ISeparate_termContext)
}

func (s *TermContext) Function_term() IFunction_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IFunction_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IFunction_termContext)
}

func (s *TermContext) Lambda_term() ILambda_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ILambda_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ILambda_termContext)
}

func (s *TermContext) Generator_term() IGenerator_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IGenerator_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IGenerator_termContext)
}

func (s *TermContext) Index_term() IIndex_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IIndex_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IIndex_termContext)
}

func (s *TermContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *TermContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *TermContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterTerm(s)
	}
}

func (s *TermContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitTerm(s)
	}
}

func (p *BundParser) Term() (localctx ITermContext) {
	localctx = NewTermContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 8, BundParserRULE_term)

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(98)
	p.GetErrorHandler().Sync(p)
	switch p.GetInterpreter().AdaptivePredict(p.GetTokenStream(), 5, p.GetParserRuleContext()) {
	case 1:
		{
			p.SetState(80)
			p.Ns()
		}

	case 2:
		{
			p.SetState(81)
			p.Block()
		}

	case 3:
		{
			p.SetState(82)
			p.Call_term()
		}

	case 4:
		{
			p.SetState(83)
			p.Ref_call_term()
		}

	case 5:
		{
			p.SetState(84)
			p.Boolean_term()
		}

	case 6:
		{
			p.SetState(85)
			p.Integer_term()
		}

	case 7:
		{
			p.SetState(86)
			p.Float_term()
		}

	case 8:
		{
			p.SetState(87)
			p.String_term()
		}

	case 9:
		{
			p.SetState(88)
			p.Complex_term()
		}

	case 10:
		{
			p.SetState(89)
			p.Datablock_term()
		}

	case 11:
		{
			p.SetState(90)
			p.Matchblock_term()
		}

	case 12:
		{
			p.SetState(91)
			p.Logicblock_term()
		}

	case 13:
		{
			p.SetState(92)
			p.Mode_term()
		}

	case 14:
		{
			p.SetState(93)
			p.Separate_term()
		}

	case 15:
		{
			p.SetState(94)
			p.Function_term()
		}

	case 16:
		{
			p.SetState(95)
			p.Lambda_term()
		}

	case 17:
		{
			p.SetState(96)
			p.Generator_term()
		}

	case 18:
		{
			p.SetState(97)
			p.Index_term()
		}

	}

	return localctx
}

// IDataContext is an interface to support dynamic dispatch.
type IDataContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// IsDataContext differentiates from other interfaces.
	IsDataContext()
}

type DataContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
}

func NewEmptyDataContext() *DataContext {
	var p = new(DataContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_data
	return p
}

func (*DataContext) IsDataContext() {}

func NewDataContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *DataContext {
	var p = new(DataContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_data

	return p
}

func (s *DataContext) GetParser() antlr.Parser { return s.parser }

func (s *DataContext) Boolean_term() IBoolean_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IBoolean_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IBoolean_termContext)
}

func (s *DataContext) Integer_term() IInteger_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IInteger_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IInteger_termContext)
}

func (s *DataContext) Float_term() IFloat_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IFloat_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IFloat_termContext)
}

func (s *DataContext) String_term() IString_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IString_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IString_termContext)
}

func (s *DataContext) Complex_term() IComplex_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IComplex_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IComplex_termContext)
}

func (s *DataContext) Call_term() ICall_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ICall_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ICall_termContext)
}

func (s *DataContext) Separate_term() ISeparate_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ISeparate_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ISeparate_termContext)
}

func (s *DataContext) Lambda_term() ILambda_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ILambda_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(ILambda_termContext)
}

func (s *DataContext) Ref_call_term() IRef_call_termContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IRef_call_termContext)(nil)).Elem(), 0)

	if t == nil {
		return nil
	}

	return t.(IRef_call_termContext)
}

func (s *DataContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *DataContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *DataContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterData(s)
	}
}

func (s *DataContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitData(s)
	}
}

func (p *BundParser) Data() (localctx IDataContext) {
	localctx = NewDataContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 10, BundParserRULE_data)

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(109)
	p.GetErrorHandler().Sync(p)
	switch p.GetInterpreter().AdaptivePredict(p.GetTokenStream(), 6, p.GetParserRuleContext()) {
	case 1:
		{
			p.SetState(100)
			p.Boolean_term()
		}

	case 2:
		{
			p.SetState(101)
			p.Integer_term()
		}

	case 3:
		{
			p.SetState(102)
			p.Float_term()
		}

	case 4:
		{
			p.SetState(103)
			p.String_term()
		}

	case 5:
		{
			p.SetState(104)
			p.Complex_term()
		}

	case 6:
		{
			p.SetState(105)
			p.Call_term()
		}

	case 7:
		{
			p.SetState(106)
			p.Separate_term()
		}

	case 8:
		{
			p.SetState(107)
			p.Lambda_term()
		}

	case 9:
		{
			p.SetState(108)
			p.Ref_call_term()
		}

	}

	return localctx
}

// ICall_termContext is an interface to support dynamic dispatch.
type ICall_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsCall_termContext differentiates from other interfaces.
	IsCall_termContext()
}

type Call_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyCall_termContext() *Call_termContext {
	var p = new(Call_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_call_term
	return p
}

func (*Call_termContext) IsCall_termContext() {}

func NewCall_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Call_termContext {
	var p = new(Call_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_call_term

	return p
}

func (s *Call_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Call_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *Call_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Call_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Call_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *Call_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Call_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Call_termContext) AllSYS() []antlr.TerminalNode {
	return s.GetTokens(BundParserSYS)
}

func (s *Call_termContext) SYS(i int) antlr.TerminalNode {
	return s.GetToken(BundParserSYS, i)
}

func (s *Call_termContext) AllSYSF() []antlr.TerminalNode {
	return s.GetTokens(BundParserSYSF)
}

func (s *Call_termContext) SYSF(i int) antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, i)
}

func (s *Call_termContext) AllCMD() []antlr.TerminalNode {
	return s.GetTokens(BundParserCMD)
}

func (s *Call_termContext) CMD(i int) antlr.TerminalNode {
	return s.GetToken(BundParserCMD, i)
}

func (s *Call_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *Call_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *Call_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Call_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Call_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterCall_term(s)
	}
}

func (s *Call_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitCall_term(s)
	}
}

func (p *BundParser) Call_term() (localctx ICall_termContext) {
	localctx = NewCall_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 12, BundParserRULE_call_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(113)
	p.GetErrorHandler().Sync(p)

	if p.GetInterpreter().AdaptivePredict(p.GetTokenStream(), 7, p.GetParserRuleContext()) == 1 {
		{
			p.SetState(111)

			var _m = p.Match(BundParserNAME)

			localctx.(*Call_termContext).PRE = _m
		}
		{
			p.SetState(112)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(115)

		var _lt = p.GetTokenStream().LT(1)

		localctx.(*Call_termContext).VALUE = _lt

		_la = p.GetTokenStream().LA(1)

		if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
			var _ri = p.GetErrorHandler().RecoverInline(p)

			localctx.(*Call_termContext).VALUE = _ri
		} else {
			p.GetErrorHandler().ReportMatch(p)
			p.Consume()
		}
	}
	p.SetState(119)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(116)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(117)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Call_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Call_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(118)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IRef_call_termContext is an interface to support dynamic dispatch.
type IRef_call_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsRef_call_termContext differentiates from other interfaces.
	IsRef_call_termContext()
}

type Ref_call_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyRef_call_termContext() *Ref_call_termContext {
	var p = new(Ref_call_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_ref_call_term
	return p
}

func (*Ref_call_termContext) IsRef_call_termContext() {}

func NewRef_call_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Ref_call_termContext {
	var p = new(Ref_call_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_ref_call_term

	return p
}

func (s *Ref_call_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Ref_call_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *Ref_call_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Ref_call_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Ref_call_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *Ref_call_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Ref_call_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Ref_call_termContext) AllSYS() []antlr.TerminalNode {
	return s.GetTokens(BundParserSYS)
}

func (s *Ref_call_termContext) SYS(i int) antlr.TerminalNode {
	return s.GetToken(BundParserSYS, i)
}

func (s *Ref_call_termContext) AllSYSF() []antlr.TerminalNode {
	return s.GetTokens(BundParserSYSF)
}

func (s *Ref_call_termContext) SYSF(i int) antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, i)
}

func (s *Ref_call_termContext) AllCMD() []antlr.TerminalNode {
	return s.GetTokens(BundParserCMD)
}

func (s *Ref_call_termContext) CMD(i int) antlr.TerminalNode {
	return s.GetToken(BundParserCMD, i)
}

func (s *Ref_call_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *Ref_call_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *Ref_call_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Ref_call_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Ref_call_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterRef_call_term(s)
	}
}

func (s *Ref_call_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitRef_call_term(s)
	}
}

func (p *BundParser) Ref_call_term() (localctx IRef_call_termContext) {
	localctx = NewRef_call_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 14, BundParserRULE_ref_call_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(121)
		p.Match(BundParserT__6)
	}
	p.SetState(124)
	p.GetErrorHandler().Sync(p)

	if p.GetInterpreter().AdaptivePredict(p.GetTokenStream(), 9, p.GetParserRuleContext()) == 1 {
		{
			p.SetState(122)

			var _m = p.Match(BundParserNAME)

			localctx.(*Ref_call_termContext).PRE = _m
		}
		{
			p.SetState(123)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(126)

		var _lt = p.GetTokenStream().LT(1)

		localctx.(*Ref_call_termContext).VALUE = _lt

		_la = p.GetTokenStream().LA(1)

		if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
			var _ri = p.GetErrorHandler().RecoverInline(p)

			localctx.(*Ref_call_termContext).VALUE = _ri
		} else {
			p.GetErrorHandler().ReportMatch(p)
			p.Consume()
		}
	}
	p.SetState(130)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(127)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(128)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Ref_call_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Ref_call_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(129)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IBoolean_termContext is an interface to support dynamic dispatch.
type IBoolean_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsBoolean_termContext differentiates from other interfaces.
	IsBoolean_termContext()
}

type Boolean_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyBoolean_termContext() *Boolean_termContext {
	var p = new(Boolean_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_boolean_term
	return p
}

func (*Boolean_termContext) IsBoolean_termContext() {}

func NewBoolean_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Boolean_termContext {
	var p = new(Boolean_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_boolean_term

	return p
}

func (s *Boolean_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Boolean_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *Boolean_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Boolean_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Boolean_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *Boolean_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Boolean_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Boolean_termContext) TRUE() antlr.TerminalNode {
	return s.GetToken(BundParserTRUE, 0)
}

func (s *Boolean_termContext) FALSE() antlr.TerminalNode {
	return s.GetToken(BundParserFALSE, 0)
}

func (s *Boolean_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *Boolean_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *Boolean_termContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *Boolean_termContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *Boolean_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Boolean_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Boolean_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Boolean_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterBoolean_term(s)
	}
}

func (s *Boolean_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitBoolean_term(s)
	}
}

func (p *BundParser) Boolean_term() (localctx IBoolean_termContext) {
	localctx = NewBoolean_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 16, BundParserRULE_boolean_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(134)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserNAME {
		{
			p.SetState(132)

			var _m = p.Match(BundParserNAME)

			localctx.(*Boolean_termContext).PRE = _m
		}
		{
			p.SetState(133)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(136)

		var _lt = p.GetTokenStream().LT(1)

		localctx.(*Boolean_termContext).VALUE = _lt

		_la = p.GetTokenStream().LA(1)

		if !(_la == BundParserTRUE || _la == BundParserFALSE) {
			var _ri = p.GetErrorHandler().RecoverInline(p)

			localctx.(*Boolean_termContext).VALUE = _ri
		} else {
			p.GetErrorHandler().ReportMatch(p)
			p.Consume()
		}
	}
	p.SetState(140)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(137)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(138)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Boolean_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Boolean_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(139)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IInteger_termContext is an interface to support dynamic dispatch.
type IInteger_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsInteger_termContext differentiates from other interfaces.
	IsInteger_termContext()
}

type Integer_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyInteger_termContext() *Integer_termContext {
	var p = new(Integer_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_integer_term
	return p
}

func (*Integer_termContext) IsInteger_termContext() {}

func NewInteger_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Integer_termContext {
	var p = new(Integer_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_integer_term

	return p
}

func (s *Integer_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Integer_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *Integer_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Integer_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Integer_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *Integer_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Integer_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Integer_termContext) INTEGER() antlr.TerminalNode {
	return s.GetToken(BundParserINTEGER, 0)
}

func (s *Integer_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *Integer_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *Integer_termContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *Integer_termContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *Integer_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Integer_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Integer_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Integer_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterInteger_term(s)
	}
}

func (s *Integer_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitInteger_term(s)
	}
}

func (p *BundParser) Integer_term() (localctx IInteger_termContext) {
	localctx = NewInteger_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 18, BundParserRULE_integer_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(144)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserNAME {
		{
			p.SetState(142)

			var _m = p.Match(BundParserNAME)

			localctx.(*Integer_termContext).PRE = _m
		}
		{
			p.SetState(143)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(146)

		var _m = p.Match(BundParserINTEGER)

		localctx.(*Integer_termContext).VALUE = _m
	}
	p.SetState(150)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(147)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(148)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Integer_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Integer_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(149)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IFloat_termContext is an interface to support dynamic dispatch.
type IFloat_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsFloat_termContext differentiates from other interfaces.
	IsFloat_termContext()
}

type Float_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyFloat_termContext() *Float_termContext {
	var p = new(Float_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_float_term
	return p
}

func (*Float_termContext) IsFloat_termContext() {}

func NewFloat_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Float_termContext {
	var p = new(Float_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_float_term

	return p
}

func (s *Float_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Float_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *Float_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Float_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Float_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *Float_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Float_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Float_termContext) FLOAT_NUMBER() antlr.TerminalNode {
	return s.GetToken(BundParserFLOAT_NUMBER, 0)
}

func (s *Float_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *Float_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *Float_termContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *Float_termContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *Float_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Float_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Float_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Float_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterFloat_term(s)
	}
}

func (s *Float_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitFloat_term(s)
	}
}

func (p *BundParser) Float_term() (localctx IFloat_termContext) {
	localctx = NewFloat_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 20, BundParserRULE_float_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(154)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserNAME {
		{
			p.SetState(152)

			var _m = p.Match(BundParserNAME)

			localctx.(*Float_termContext).PRE = _m
		}
		{
			p.SetState(153)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(156)

		var _lt = p.GetTokenStream().LT(1)

		localctx.(*Float_termContext).VALUE = _lt

		_la = p.GetTokenStream().LA(1)

		if !(((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserFLOAT_NUMBER))) != 0) {
			var _ri = p.GetErrorHandler().RecoverInline(p)

			localctx.(*Float_termContext).VALUE = _ri
		} else {
			p.GetErrorHandler().ReportMatch(p)
			p.Consume()
		}
	}
	p.SetState(160)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(157)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(158)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Float_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Float_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(159)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IString_termContext is an interface to support dynamic dispatch.
type IString_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsString_termContext differentiates from other interfaces.
	IsString_termContext()
}

type String_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyString_termContext() *String_termContext {
	var p = new(String_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_string_term
	return p
}

func (*String_termContext) IsString_termContext() {}

func NewString_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *String_termContext {
	var p = new(String_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_string_term

	return p
}

func (s *String_termContext) GetParser() antlr.Parser { return s.parser }

func (s *String_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *String_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *String_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *String_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *String_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *String_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *String_termContext) STRING() antlr.TerminalNode {
	return s.GetToken(BundParserSTRING, 0)
}

func (s *String_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *String_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *String_termContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *String_termContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *String_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *String_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *String_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *String_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterString_term(s)
	}
}

func (s *String_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitString_term(s)
	}
}

func (p *BundParser) String_term() (localctx IString_termContext) {
	localctx = NewString_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 22, BundParserRULE_string_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(164)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserNAME {
		{
			p.SetState(162)

			var _m = p.Match(BundParserNAME)

			localctx.(*String_termContext).PRE = _m
		}
		{
			p.SetState(163)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(166)

		var _m = p.Match(BundParserSTRING)

		localctx.(*String_termContext).VALUE = _m
	}
	p.SetState(170)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(167)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(168)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*String_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*String_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(169)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IComplex_termContext is an interface to support dynamic dispatch.
type IComplex_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetPRE returns the PRE token.
	GetPRE() antlr.Token

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetPRE sets the PRE token.
	SetPRE(antlr.Token)

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// IsComplex_termContext differentiates from other interfaces.
	IsComplex_termContext()
}

type Complex_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	PRE     antlr.Token
	VALUE   antlr.Token
	FUNCTOR antlr.Token
}

func NewEmptyComplex_termContext() *Complex_termContext {
	var p = new(Complex_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_complex_term
	return p
}

func (*Complex_termContext) IsComplex_termContext() {}

func NewComplex_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Complex_termContext {
	var p = new(Complex_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_complex_term

	return p
}

func (s *Complex_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Complex_termContext) GetPRE() antlr.Token { return s.PRE }

func (s *Complex_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Complex_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Complex_termContext) SetPRE(v antlr.Token) { s.PRE = v }

func (s *Complex_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Complex_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Complex_termContext) COMPLEX_NUMBER() antlr.TerminalNode {
	return s.GetToken(BundParserCOMPLEX_NUMBER, 0)
}

func (s *Complex_termContext) AllNAME() []antlr.TerminalNode {
	return s.GetTokens(BundParserNAME)
}

func (s *Complex_termContext) NAME(i int) antlr.TerminalNode {
	return s.GetToken(BundParserNAME, i)
}

func (s *Complex_termContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *Complex_termContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *Complex_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Complex_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Complex_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Complex_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterComplex_term(s)
	}
}

func (s *Complex_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitComplex_term(s)
	}
}

func (p *BundParser) Complex_term() (localctx IComplex_termContext) {
	localctx = NewComplex_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 24, BundParserRULE_complex_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	p.SetState(174)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserNAME {
		{
			p.SetState(172)

			var _m = p.Match(BundParserNAME)

			localctx.(*Complex_termContext).PRE = _m
		}
		{
			p.SetState(173)
			p.Match(BundParserT__5)
		}

	}
	{
		p.SetState(176)

		var _m = p.Match(BundParserCOMPLEX_NUMBER)

		localctx.(*Complex_termContext).VALUE = _m
	}
	p.SetState(180)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(177)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(178)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Complex_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Complex_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(179)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IMode_termContext is an interface to support dynamic dispatch.
type IMode_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// IsMode_termContext differentiates from other interfaces.
	IsMode_termContext()
}

type Mode_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	VALUE  antlr.Token
}

func NewEmptyMode_termContext() *Mode_termContext {
	var p = new(Mode_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_mode_term
	return p
}

func (*Mode_termContext) IsMode_termContext() {}

func NewMode_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Mode_termContext {
	var p = new(Mode_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_mode_term

	return p
}

func (s *Mode_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Mode_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Mode_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Mode_termContext) TOBEGIN() antlr.TerminalNode {
	return s.GetToken(BundParserTOBEGIN, 0)
}

func (s *Mode_termContext) TOEND() antlr.TerminalNode {
	return s.GetToken(BundParserTOEND, 0)
}

func (s *Mode_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Mode_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Mode_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterMode_term(s)
	}
}

func (s *Mode_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitMode_term(s)
	}
}

func (p *BundParser) Mode_term() (localctx IMode_termContext) {
	localctx = NewMode_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 26, BundParserRULE_mode_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(182)

		var _lt = p.GetTokenStream().LT(1)

		localctx.(*Mode_termContext).VALUE = _lt

		_la = p.GetTokenStream().LA(1)

		if !(_la == BundParserTOBEGIN || _la == BundParserTOEND) {
			var _ri = p.GetErrorHandler().RecoverInline(p)

			localctx.(*Mode_termContext).VALUE = _ri
		} else {
			p.GetErrorHandler().ReportMatch(p)
			p.Consume()
		}
	}

	return localctx
}

// ISeparate_termContext is an interface to support dynamic dispatch.
type ISeparate_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// IsSeparate_termContext differentiates from other interfaces.
	IsSeparate_termContext()
}

type Separate_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	VALUE  antlr.Token
}

func NewEmptySeparate_termContext() *Separate_termContext {
	var p = new(Separate_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_separate_term
	return p
}

func (*Separate_termContext) IsSeparate_termContext() {}

func NewSeparate_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Separate_termContext {
	var p = new(Separate_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_separate_term

	return p
}

func (s *Separate_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Separate_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Separate_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Separate_termContext) SEPARATE() antlr.TerminalNode {
	return s.GetToken(BundParserSEPARATE, 0)
}

func (s *Separate_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Separate_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Separate_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterSeparate_term(s)
	}
}

func (s *Separate_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitSeparate_term(s)
	}
}

func (p *BundParser) Separate_term() (localctx ISeparate_termContext) {
	localctx = NewSeparate_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 28, BundParserRULE_separate_term)

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(184)

		var _m = p.Match(BundParserSEPARATE)

		localctx.(*Separate_termContext).VALUE = _m
	}

	return localctx
}

// IDatablock_termContext is an interface to support dynamic dispatch.
type IDatablock_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetFUNCTOR returns the FUNCTOR token.
	GetFUNCTOR() antlr.Token

	// SetFUNCTOR sets the FUNCTOR token.
	SetFUNCTOR(antlr.Token)

	// Get_data returns the _data rule contexts.
	Get_data() IDataContext

	// Set_data sets the _data rule contexts.
	Set_data(IDataContext)

	// GetBody returns the body rule context list.
	GetBody() []IDataContext

	// SetBody sets the body rule context list.
	SetBody([]IDataContext)

	// IsDatablock_termContext differentiates from other interfaces.
	IsDatablock_termContext()
}

type Datablock_termContext struct {
	*antlr.BaseParserRuleContext
	parser  antlr.Parser
	_data   IDataContext
	body    []IDataContext
	FUNCTOR antlr.Token
}

func NewEmptyDatablock_termContext() *Datablock_termContext {
	var p = new(Datablock_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_datablock_term
	return p
}

func (*Datablock_termContext) IsDatablock_termContext() {}

func NewDatablock_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Datablock_termContext {
	var p = new(Datablock_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_datablock_term

	return p
}

func (s *Datablock_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Datablock_termContext) GetFUNCTOR() antlr.Token { return s.FUNCTOR }

func (s *Datablock_termContext) SetFUNCTOR(v antlr.Token) { s.FUNCTOR = v }

func (s *Datablock_termContext) Get_data() IDataContext { return s._data }

func (s *Datablock_termContext) Set_data(v IDataContext) { s._data = v }

func (s *Datablock_termContext) GetBody() []IDataContext { return s.body }

func (s *Datablock_termContext) SetBody(v []IDataContext) { s.body = v }

func (s *Datablock_termContext) AllData() []IDataContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*IDataContext)(nil)).Elem())
	var tst = make([]IDataContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(IDataContext)
		}
	}

	return tst
}

func (s *Datablock_termContext) Data(i int) IDataContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IDataContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(IDataContext)
}

func (s *Datablock_termContext) SYS() antlr.TerminalNode {
	return s.GetToken(BundParserSYS, 0)
}

func (s *Datablock_termContext) SYSF() antlr.TerminalNode {
	return s.GetToken(BundParserSYSF, 0)
}

func (s *Datablock_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Datablock_termContext) NAME() antlr.TerminalNode {
	return s.GetToken(BundParserNAME, 0)
}

func (s *Datablock_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Datablock_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Datablock_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterDatablock_term(s)
	}
}

func (s *Datablock_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitDatablock_term(s)
	}
}

func (p *BundParser) Datablock_term() (localctx IDatablock_termContext) {
	localctx = NewDatablock_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 30, BundParserRULE_datablock_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(186)
		p.Match(BundParserT__11)
	}
	p.SetState(190)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__17)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(187)

			var _x = p.Data()

			localctx.(*Datablock_termContext)._data = _x
		}
		localctx.(*Datablock_termContext).body = append(localctx.(*Datablock_termContext).body, localctx.(*Datablock_termContext)._data)

		p.SetState(192)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(193)
		p.Match(BundParserT__3)
	}
	p.SetState(197)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__4 {
		{
			p.SetState(194)
			p.Match(BundParserT__4)
		}
		{
			p.SetState(195)

			var _lt = p.GetTokenStream().LT(1)

			localctx.(*Datablock_termContext).FUNCTOR = _lt

			_la = p.GetTokenStream().LA(1)

			if !(((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32)))) != 0) {
				var _ri = p.GetErrorHandler().RecoverInline(p)

				localctx.(*Datablock_termContext).FUNCTOR = _ri
			} else {
				p.GetErrorHandler().ReportMatch(p)
				p.Consume()
			}
		}
		{
			p.SetState(196)
			p.Match(BundParserT__3)
		}

	}

	return localctx
}

// IMatchblock_termContext is an interface to support dynamic dispatch.
type IMatchblock_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// Get_data returns the _data rule contexts.
	Get_data() IDataContext

	// Set_data sets the _data rule contexts.
	Set_data(IDataContext)

	// GetBody returns the body rule context list.
	GetBody() []IDataContext

	// SetBody sets the body rule context list.
	SetBody([]IDataContext)

	// IsMatchblock_termContext differentiates from other interfaces.
	IsMatchblock_termContext()
}

type Matchblock_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	_data  IDataContext
	body   []IDataContext
}

func NewEmptyMatchblock_termContext() *Matchblock_termContext {
	var p = new(Matchblock_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_matchblock_term
	return p
}

func (*Matchblock_termContext) IsMatchblock_termContext() {}

func NewMatchblock_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Matchblock_termContext {
	var p = new(Matchblock_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_matchblock_term

	return p
}

func (s *Matchblock_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Matchblock_termContext) Get_data() IDataContext { return s._data }

func (s *Matchblock_termContext) Set_data(v IDataContext) { s._data = v }

func (s *Matchblock_termContext) GetBody() []IDataContext { return s.body }

func (s *Matchblock_termContext) SetBody(v []IDataContext) { s.body = v }

func (s *Matchblock_termContext) AllData() []IDataContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*IDataContext)(nil)).Elem())
	var tst = make([]IDataContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(IDataContext)
		}
	}

	return tst
}

func (s *Matchblock_termContext) Data(i int) IDataContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*IDataContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(IDataContext)
}

func (s *Matchblock_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Matchblock_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Matchblock_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterMatchblock_term(s)
	}
}

func (s *Matchblock_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitMatchblock_term(s)
	}
}

func (p *BundParser) Matchblock_term() (localctx IMatchblock_termContext) {
	localctx = NewMatchblock_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 32, BundParserRULE_matchblock_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(199)
		p.Match(BundParserT__12)
	}
	p.SetState(201)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for ok := true; ok; ok = (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__17)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(200)

			var _x = p.Data()

			localctx.(*Matchblock_termContext)._data = _x
		}
		localctx.(*Matchblock_termContext).body = append(localctx.(*Matchblock_termContext).body, localctx.(*Matchblock_termContext)._data)

		p.SetState(203)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(205)
		p.Match(BundParserT__3)
	}

	return localctx
}

// ILogicblock_termContext is an interface to support dynamic dispatch.
type ILogicblock_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetHDR returns the HDR token.
	GetHDR() antlr.Token

	// SetHDR sets the HDR token.
	SetHDR(antlr.Token)

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsLogicblock_termContext differentiates from other interfaces.
	IsLogicblock_termContext()
}

type Logicblock_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	HDR    antlr.Token
	_term  ITermContext
	body   []ITermContext
}

func NewEmptyLogicblock_termContext() *Logicblock_termContext {
	var p = new(Logicblock_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_logicblock_term
	return p
}

func (*Logicblock_termContext) IsLogicblock_termContext() {}

func NewLogicblock_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Logicblock_termContext {
	var p = new(Logicblock_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_logicblock_term

	return p
}

func (s *Logicblock_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Logicblock_termContext) GetHDR() antlr.Token { return s.HDR }

func (s *Logicblock_termContext) SetHDR(v antlr.Token) { s.HDR = v }

func (s *Logicblock_termContext) Get_term() ITermContext { return s._term }

func (s *Logicblock_termContext) Set_term(v ITermContext) { s._term = v }

func (s *Logicblock_termContext) GetBody() []ITermContext { return s.body }

func (s *Logicblock_termContext) SetBody(v []ITermContext) { s.body = v }

func (s *Logicblock_termContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *Logicblock_termContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *Logicblock_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Logicblock_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Logicblock_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterLogicblock_term(s)
	}
}

func (s *Logicblock_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitLogicblock_term(s)
	}
}

func (p *BundParser) Logicblock_term() (localctx ILogicblock_termContext) {
	localctx = NewLogicblock_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 34, BundParserRULE_logicblock_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(207)

		var _lt = p.GetTokenStream().LT(1)

		localctx.(*Logicblock_termContext).HDR = _lt

		_la = p.GetTokenStream().LA(1)

		if !(_la == BundParserT__13 || _la == BundParserT__14) {
			var _ri = p.GetErrorHandler().RecoverInline(p)

			localctx.(*Logicblock_termContext).HDR = _ri
		} else {
			p.GetErrorHandler().ReportMatch(p)
			p.Consume()
		}
	}
	p.SetState(211)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(208)

			var _x = p.Term()

			localctx.(*Logicblock_termContext)._term = _x
		}
		localctx.(*Logicblock_termContext).body = append(localctx.(*Logicblock_termContext).body, localctx.(*Logicblock_termContext)._term)

		p.SetState(213)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(214)
		p.Match(BundParserT__3)
	}

	return localctx
}

// IFunction_termContext is an interface to support dynamic dispatch.
type IFunction_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetName returns the name token.
	GetName() antlr.Token

	// SetName sets the name token.
	SetName(antlr.Token)

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsFunction_termContext differentiates from other interfaces.
	IsFunction_termContext()
}

type Function_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	name   antlr.Token
	_term  ITermContext
	body   []ITermContext
}

func NewEmptyFunction_termContext() *Function_termContext {
	var p = new(Function_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_function_term
	return p
}

func (*Function_termContext) IsFunction_termContext() {}

func NewFunction_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Function_termContext {
	var p = new(Function_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_function_term

	return p
}

func (s *Function_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Function_termContext) GetName() antlr.Token { return s.name }

func (s *Function_termContext) SetName(v antlr.Token) { s.name = v }

func (s *Function_termContext) Get_term() ITermContext { return s._term }

func (s *Function_termContext) Set_term(v ITermContext) { s._term = v }

func (s *Function_termContext) GetBody() []ITermContext { return s.body }

func (s *Function_termContext) SetBody(v []ITermContext) { s.body = v }

func (s *Function_termContext) NAME() antlr.TerminalNode {
	return s.GetToken(BundParserNAME, 0)
}

func (s *Function_termContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *Function_termContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *Function_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Function_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Function_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterFunction_term(s)
	}
}

func (s *Function_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitFunction_term(s)
	}
}

func (p *BundParser) Function_term() (localctx IFunction_termContext) {
	localctx = NewFunction_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 36, BundParserRULE_function_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(216)
		p.Match(BundParserT__0)
	}
	{
		p.SetState(217)

		var _m = p.Match(BundParserNAME)

		localctx.(*Function_termContext).name = _m
	}
	{
		p.SetState(218)
		p.Match(BundParserT__15)
	}
	p.SetState(222)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(219)

			var _x = p.Term()

			localctx.(*Function_termContext)._term = _x
		}
		localctx.(*Function_termContext).body = append(localctx.(*Function_termContext).body, localctx.(*Function_termContext)._term)

		p.SetState(224)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(225)
		p.Match(BundParserT__16)
	}

	return localctx
}

// ILambda_termContext is an interface to support dynamic dispatch.
type ILambda_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsLambda_termContext differentiates from other interfaces.
	IsLambda_termContext()
}

type Lambda_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	_term  ITermContext
	body   []ITermContext
}

func NewEmptyLambda_termContext() *Lambda_termContext {
	var p = new(Lambda_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_lambda_term
	return p
}

func (*Lambda_termContext) IsLambda_termContext() {}

func NewLambda_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Lambda_termContext {
	var p = new(Lambda_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_lambda_term

	return p
}

func (s *Lambda_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Lambda_termContext) Get_term() ITermContext { return s._term }

func (s *Lambda_termContext) Set_term(v ITermContext) { s._term = v }

func (s *Lambda_termContext) GetBody() []ITermContext { return s.body }

func (s *Lambda_termContext) SetBody(v []ITermContext) { s.body = v }

func (s *Lambda_termContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *Lambda_termContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *Lambda_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Lambda_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Lambda_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterLambda_term(s)
	}
}

func (s *Lambda_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitLambda_term(s)
	}
}

func (p *BundParser) Lambda_term() (localctx ILambda_termContext) {
	localctx = NewLambda_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 38, BundParserRULE_lambda_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(227)
		p.Match(BundParserT__17)
	}
	p.SetState(231)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(228)

			var _x = p.Term()

			localctx.(*Lambda_termContext)._term = _x
		}
		localctx.(*Lambda_termContext).body = append(localctx.(*Lambda_termContext).body, localctx.(*Lambda_termContext)._term)

		p.SetState(233)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(234)
		p.Match(BundParserT__16)
	}

	return localctx
}

// IOperation_termContext is an interface to support dynamic dispatch.
type IOperation_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetName returns the name token.
	GetName() antlr.Token

	// SetName sets the name token.
	SetName(antlr.Token)

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsOperation_termContext differentiates from other interfaces.
	IsOperation_termContext()
}

type Operation_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	name   antlr.Token
	_term  ITermContext
	body   []ITermContext
}

func NewEmptyOperation_termContext() *Operation_termContext {
	var p = new(Operation_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_operation_term
	return p
}

func (*Operation_termContext) IsOperation_termContext() {}

func NewOperation_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Operation_termContext {
	var p = new(Operation_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_operation_term

	return p
}

func (s *Operation_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Operation_termContext) GetName() antlr.Token { return s.name }

func (s *Operation_termContext) SetName(v antlr.Token) { s.name = v }

func (s *Operation_termContext) Get_term() ITermContext { return s._term }

func (s *Operation_termContext) Set_term(v ITermContext) { s._term = v }

func (s *Operation_termContext) GetBody() []ITermContext { return s.body }

func (s *Operation_termContext) SetBody(v []ITermContext) { s.body = v }

func (s *Operation_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Operation_termContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *Operation_termContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *Operation_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Operation_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Operation_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterOperation_term(s)
	}
}

func (s *Operation_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitOperation_term(s)
	}
}

func (p *BundParser) Operation_term() (localctx IOperation_termContext) {
	localctx = NewOperation_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 40, BundParserRULE_operation_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(236)
		p.Match(BundParserT__18)
	}
	{
		p.SetState(237)

		var _m = p.Match(BundParserCMD)

		localctx.(*Operation_termContext).name = _m
	}
	{
		p.SetState(238)
		p.Match(BundParserT__19)
	}
	p.SetState(242)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(239)

			var _x = p.Term()

			localctx.(*Operation_termContext)._term = _x
		}
		localctx.(*Operation_termContext).body = append(localctx.(*Operation_termContext).body, localctx.(*Operation_termContext)._term)

		p.SetState(244)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(245)
		p.Match(BundParserT__16)
	}

	return localctx
}

// IGenerator_termContext is an interface to support dynamic dispatch.
type IGenerator_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetName returns the name token.
	GetName() antlr.Token

	// SetName sets the name token.
	SetName(antlr.Token)

	// Get_term returns the _term rule contexts.
	Get_term() ITermContext

	// Set_term sets the _term rule contexts.
	Set_term(ITermContext)

	// GetBody returns the body rule context list.
	GetBody() []ITermContext

	// SetBody sets the body rule context list.
	SetBody([]ITermContext)

	// IsGenerator_termContext differentiates from other interfaces.
	IsGenerator_termContext()
}

type Generator_termContext struct {
	*antlr.BaseParserRuleContext
	parser antlr.Parser
	name   antlr.Token
	_term  ITermContext
	body   []ITermContext
}

func NewEmptyGenerator_termContext() *Generator_termContext {
	var p = new(Generator_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_generator_term
	return p
}

func (*Generator_termContext) IsGenerator_termContext() {}

func NewGenerator_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Generator_termContext {
	var p = new(Generator_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_generator_term

	return p
}

func (s *Generator_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Generator_termContext) GetName() antlr.Token { return s.name }

func (s *Generator_termContext) SetName(v antlr.Token) { s.name = v }

func (s *Generator_termContext) Get_term() ITermContext { return s._term }

func (s *Generator_termContext) Set_term(v ITermContext) { s._term = v }

func (s *Generator_termContext) GetBody() []ITermContext { return s.body }

func (s *Generator_termContext) SetBody(v []ITermContext) { s.body = v }

func (s *Generator_termContext) CMD() antlr.TerminalNode {
	return s.GetToken(BundParserCMD, 0)
}

func (s *Generator_termContext) AllTerm() []ITermContext {
	var ts = s.GetTypedRuleContexts(reflect.TypeOf((*ITermContext)(nil)).Elem())
	var tst = make([]ITermContext, len(ts))

	for i, t := range ts {
		if t != nil {
			tst[i] = t.(ITermContext)
		}
	}

	return tst
}

func (s *Generator_termContext) Term(i int) ITermContext {
	var t = s.GetTypedRuleContext(reflect.TypeOf((*ITermContext)(nil)).Elem(), i)

	if t == nil {
		return nil
	}

	return t.(ITermContext)
}

func (s *Generator_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Generator_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Generator_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterGenerator_term(s)
	}
}

func (s *Generator_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitGenerator_term(s)
	}
}

func (p *BundParser) Generator_term() (localctx IGenerator_termContext) {
	localctx = NewGenerator_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 42, BundParserRULE_generator_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(247)
		p.Match(BundParserT__20)
	}
	{
		p.SetState(248)

		var _m = p.Match(BundParserCMD)

		localctx.(*Generator_termContext).name = _m
	}
	{
		p.SetState(249)
		p.Match(BundParserT__21)
	}
	p.SetState(253)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	for (((_la)&-(0x1f+1)) == 0 && ((1<<uint(_la))&((1<<BundParserT__0)|(1<<BundParserT__2)|(1<<BundParserT__6)|(1<<BundParserT__7)|(1<<BundParserT__8)|(1<<BundParserT__9)|(1<<BundParserT__10)|(1<<BundParserT__11)|(1<<BundParserT__12)|(1<<BundParserT__13)|(1<<BundParserT__14)|(1<<BundParserT__17)|(1<<BundParserT__20)|(1<<BundParserT__22)|(1<<BundParserINTEGER)|(1<<BundParserFLOAT_NUMBER)|(1<<BundParserSTRING)|(1<<BundParserCOMPLEX_NUMBER)|(1<<BundParserTRUE)|(1<<BundParserFALSE))) != 0) || (((_la-32)&-(0x1f+1)) == 0 && ((1<<uint((_la-32)))&((1<<(BundParserSYS-32))|(1<<(BundParserCMD-32))|(1<<(BundParserSYSF-32))|(1<<(BundParserNAME-32))|(1<<(BundParserTOBEGIN-32))|(1<<(BundParserTOEND-32))|(1<<(BundParserSEPARATE-32)))) != 0) {
		{
			p.SetState(250)

			var _x = p.Term()

			localctx.(*Generator_termContext)._term = _x
		}
		localctx.(*Generator_termContext).body = append(localctx.(*Generator_termContext).body, localctx.(*Generator_termContext)._term)

		p.SetState(255)
		p.GetErrorHandler().Sync(p)
		_la = p.GetTokenStream().LA(1)
	}
	{
		p.SetState(256)
		p.Match(BundParserT__16)
	}

	return localctx
}

// IIndex_termContext is an interface to support dynamic dispatch.
type IIndex_termContext interface {
	antlr.ParserRuleContext

	// GetParser returns the parser.
	GetParser() antlr.Parser

	// GetVALUE returns the VALUE token.
	GetVALUE() antlr.Token

	// GetENDVALUE returns the ENDVALUE token.
	GetENDVALUE() antlr.Token

	// SetVALUE sets the VALUE token.
	SetVALUE(antlr.Token)

	// SetENDVALUE sets the ENDVALUE token.
	SetENDVALUE(antlr.Token)

	// IsIndex_termContext differentiates from other interfaces.
	IsIndex_termContext()
}

type Index_termContext struct {
	*antlr.BaseParserRuleContext
	parser   antlr.Parser
	VALUE    antlr.Token
	ENDVALUE antlr.Token
}

func NewEmptyIndex_termContext() *Index_termContext {
	var p = new(Index_termContext)
	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(nil, -1)
	p.RuleIndex = BundParserRULE_index_term
	return p
}

func (*Index_termContext) IsIndex_termContext() {}

func NewIndex_termContext(parser antlr.Parser, parent antlr.ParserRuleContext, invokingState int) *Index_termContext {
	var p = new(Index_termContext)

	p.BaseParserRuleContext = antlr.NewBaseParserRuleContext(parent, invokingState)

	p.parser = parser
	p.RuleIndex = BundParserRULE_index_term

	return p
}

func (s *Index_termContext) GetParser() antlr.Parser { return s.parser }

func (s *Index_termContext) GetVALUE() antlr.Token { return s.VALUE }

func (s *Index_termContext) GetENDVALUE() antlr.Token { return s.ENDVALUE }

func (s *Index_termContext) SetVALUE(v antlr.Token) { s.VALUE = v }

func (s *Index_termContext) SetENDVALUE(v antlr.Token) { s.ENDVALUE = v }

func (s *Index_termContext) AllINTEGER() []antlr.TerminalNode {
	return s.GetTokens(BundParserINTEGER)
}

func (s *Index_termContext) INTEGER(i int) antlr.TerminalNode {
	return s.GetToken(BundParserINTEGER, i)
}

func (s *Index_termContext) GetRuleContext() antlr.RuleContext {
	return s
}

func (s *Index_termContext) ToStringTree(ruleNames []string, recog antlr.Recognizer) string {
	return antlr.TreesStringTree(s, ruleNames, recog)
}

func (s *Index_termContext) EnterRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.EnterIndex_term(s)
	}
}

func (s *Index_termContext) ExitRule(listener antlr.ParseTreeListener) {
	if listenerT, ok := listener.(BundListener); ok {
		listenerT.ExitIndex_term(s)
	}
}

func (p *BundParser) Index_term() (localctx IIndex_termContext) {
	localctx = NewIndex_termContext(p, p.GetParserRuleContext(), p.GetState())
	p.EnterRule(localctx, 44, BundParserRULE_index_term)
	var _la int

	defer func() {
		p.ExitRule()
	}()

	defer func() {
		if err := recover(); err != nil {
			if v, ok := err.(antlr.RecognitionException); ok {
				localctx.SetException(v)
				p.GetErrorHandler().ReportError(p, v)
				p.GetErrorHandler().Recover(p, v)
			} else {
				panic(err)
			}
		}
	}()

	p.EnterOuterAlt(localctx, 1)
	{
		p.SetState(258)
		p.Match(BundParserT__22)
	}
	{
		p.SetState(259)

		var _m = p.Match(BundParserINTEGER)

		localctx.(*Index_termContext).VALUE = _m
	}
	p.SetState(262)
	p.GetErrorHandler().Sync(p)
	_la = p.GetTokenStream().LA(1)

	if _la == BundParserT__23 {
		{
			p.SetState(260)
			p.Match(BundParserT__23)
		}
		{
			p.SetState(261)

			var _m = p.Match(BundParserINTEGER)

			localctx.(*Index_termContext).ENDVALUE = _m
		}

	}

	return localctx
}
