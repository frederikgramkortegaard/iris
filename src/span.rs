/// Source location representing a span in the source code
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start_row: usize,
    pub start_column: usize,
    pub end_row: usize,
    pub end_column: usize,
}

impl Span {
    pub fn new(start_row: usize, start_column: usize, end_row: usize, end_column: usize) -> Self {
        Span {
            start_row,
            start_column,
            end_row,
            end_column,
        }
    }

    pub fn from_token(token: &crate::frontend::Token) -> Self {
        Span {
            start_row: token.row,
            start_column: token.column,
            end_row: token.row,
            end_column: token.column + token.lexeme.len(),
        }
    }

    pub fn merge(start: &Span, end: &Span) -> Self {
        Span {
            start_row: start.start_row,
            start_column: start.start_column,
            end_row: end.end_row,
            end_column: end.end_column,
        }
    }
}
