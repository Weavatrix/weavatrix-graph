use crate::{Edge, GraphError, Node, Result, SourceSpan};

pub(crate) fn validate_node(node: &Node) -> Result<()> {
    if let Some(span) = &node.span {
        validate_span(span)?;
    }
    if let Some(language) = &node.language
        && (language.is_empty() || language.trim() != language)
    {
        return Err(GraphError::InvalidKind {
            category: "language",
            value: crate::String::from(language),
        });
    }
    Ok(())
}

pub(crate) fn validate_edge(edge: &Edge) -> Result<()> {
    if edge.provenance.extractor.is_empty() {
        return Err(GraphError::EmptyExtractor);
    }
    if let Some(span) = &edge.provenance.span {
        validate_span(span)?;
    }
    Ok(())
}

fn validate_span(span: &SourceSpan) -> Result<()> {
    if span.file.is_empty() {
        return Err(GraphError::InvalidSpan {
            file: span.file.clone(),
            reason: "file must not be empty",
        });
    }
    if span.start.line == 0 || span.start.column == 0 || span.end.line == 0 || span.end.column == 0
    {
        return Err(GraphError::InvalidSpan {
            file: span.file.clone(),
            reason: "positions are one-based",
        });
    }
    if span.end < span.start {
        return Err(GraphError::InvalidSpan {
            file: span.file.clone(),
            reason: "end precedes start",
        });
    }
    Ok(())
}
