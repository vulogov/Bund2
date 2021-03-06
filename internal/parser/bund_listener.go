// Code generated from Bund.g4 by ANTLR 4.9.1. DO NOT EDIT.

package parser // Bund

import "github.com/antlr/antlr4/runtime/Go/antlr"

// BundListener is a complete listener for a parse tree produced by BundParser.
type BundListener interface {
	antlr.ParseTreeListener

	// EnterExpressions is called when entering the expressions production.
	EnterExpressions(c *ExpressionsContext)

	// EnterRoot_term is called when entering the root_term production.
	EnterRoot_term(c *Root_termContext)

	// EnterNs is called when entering the ns production.
	EnterNs(c *NsContext)

	// EnterBlock is called when entering the block production.
	EnterBlock(c *BlockContext)

	// EnterTerm is called when entering the term production.
	EnterTerm(c *TermContext)

	// EnterFterm is called when entering the fterm production.
	EnterFterm(c *FtermContext)

	// EnterData is called when entering the data production.
	EnterData(c *DataContext)

	// EnterCall_term is called when entering the call_term production.
	EnterCall_term(c *Call_termContext)

	// EnterRef_call_term is called when entering the ref_call_term production.
	EnterRef_call_term(c *Ref_call_termContext)

	// EnterBoolean_term is called when entering the boolean_term production.
	EnterBoolean_term(c *Boolean_termContext)

	// EnterInteger_term is called when entering the integer_term production.
	EnterInteger_term(c *Integer_termContext)

	// EnterFloat_term is called when entering the float_term production.
	EnterFloat_term(c *Float_termContext)

	// EnterString_term is called when entering the string_term production.
	EnterString_term(c *String_termContext)

	// EnterComplex_term is called when entering the complex_term production.
	EnterComplex_term(c *Complex_termContext)

	// EnterMode_term is called when entering the mode_term production.
	EnterMode_term(c *Mode_termContext)

	// EnterSeparate_term is called when entering the separate_term production.
	EnterSeparate_term(c *Separate_termContext)

	// EnterDatablock_term is called when entering the datablock_term production.
	EnterDatablock_term(c *Datablock_termContext)

	// EnterMatchblock_term is called when entering the matchblock_term production.
	EnterMatchblock_term(c *Matchblock_termContext)

	// EnterLogicblock_term is called when entering the logicblock_term production.
	EnterLogicblock_term(c *Logicblock_termContext)

	// EnterFunction_term is called when entering the function_term production.
	EnterFunction_term(c *Function_termContext)

	// EnterLambda_term is called when entering the lambda_term production.
	EnterLambda_term(c *Lambda_termContext)

	// EnterOperator_term is called when entering the operator_term production.
	EnterOperator_term(c *Operator_termContext)

	// EnterGenerator_term is called when entering the generator_term production.
	EnterGenerator_term(c *Generator_termContext)

	// EnterIndex_term is called when entering the index_term production.
	EnterIndex_term(c *Index_termContext)

	// ExitExpressions is called when exiting the expressions production.
	ExitExpressions(c *ExpressionsContext)

	// ExitRoot_term is called when exiting the root_term production.
	ExitRoot_term(c *Root_termContext)

	// ExitNs is called when exiting the ns production.
	ExitNs(c *NsContext)

	// ExitBlock is called when exiting the block production.
	ExitBlock(c *BlockContext)

	// ExitTerm is called when exiting the term production.
	ExitTerm(c *TermContext)

	// ExitFterm is called when exiting the fterm production.
	ExitFterm(c *FtermContext)

	// ExitData is called when exiting the data production.
	ExitData(c *DataContext)

	// ExitCall_term is called when exiting the call_term production.
	ExitCall_term(c *Call_termContext)

	// ExitRef_call_term is called when exiting the ref_call_term production.
	ExitRef_call_term(c *Ref_call_termContext)

	// ExitBoolean_term is called when exiting the boolean_term production.
	ExitBoolean_term(c *Boolean_termContext)

	// ExitInteger_term is called when exiting the integer_term production.
	ExitInteger_term(c *Integer_termContext)

	// ExitFloat_term is called when exiting the float_term production.
	ExitFloat_term(c *Float_termContext)

	// ExitString_term is called when exiting the string_term production.
	ExitString_term(c *String_termContext)

	// ExitComplex_term is called when exiting the complex_term production.
	ExitComplex_term(c *Complex_termContext)

	// ExitMode_term is called when exiting the mode_term production.
	ExitMode_term(c *Mode_termContext)

	// ExitSeparate_term is called when exiting the separate_term production.
	ExitSeparate_term(c *Separate_termContext)

	// ExitDatablock_term is called when exiting the datablock_term production.
	ExitDatablock_term(c *Datablock_termContext)

	// ExitMatchblock_term is called when exiting the matchblock_term production.
	ExitMatchblock_term(c *Matchblock_termContext)

	// ExitLogicblock_term is called when exiting the logicblock_term production.
	ExitLogicblock_term(c *Logicblock_termContext)

	// ExitFunction_term is called when exiting the function_term production.
	ExitFunction_term(c *Function_termContext)

	// ExitLambda_term is called when exiting the lambda_term production.
	ExitLambda_term(c *Lambda_termContext)

	// ExitOperator_term is called when exiting the operator_term production.
	ExitOperator_term(c *Operator_termContext)

	// ExitGenerator_term is called when exiting the generator_term production.
	ExitGenerator_term(c *Generator_termContext)

	// ExitIndex_term is called when exiting the index_term production.
	ExitIndex_term(c *Index_termContext)
}
