// Included in transcript::tests; exercise the real slot store, not a model.
mod highlight_key_regressions {
    use super::*;

    fn document(key: DocumentHighlightKey) -> Arc<zeron_syntax::HighlightedDocument> {
        Arc::new(zeron_syntax::HighlightedDocument {
            language: key.language,
            lines: Vec::new(),
        })
    }

    #[gpui::test]
    fn unchanged_ready_and_pending_fences_hash_zero_source_bytes(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let state = cx.new(|_| AppState::new());
            let transcript = cx.new(|cx| Transcript::new(state, cx));
            for lines in [100, 1_000, 10_000] {
                let source = format!("```rust\n{}```\n", "let value = 1;\n".repeat(lines));
                let tree = Arc::new(parse_full(&source));
                let key = tree.blocks[0].code_highlight_key.unwrap();
                let Block::CodeBlock { code, .. } = &tree.blocks[0].block else { panic!() };
                let before = crate::perf_trace::snapshot().highlight_hash_bytes;
                for _ in 0..50 { let _ = DocumentHighlightKey::new(key.language, code); }
                let baseline = crate::perf_trace::snapshot().highlight_hash_bytes - before;
                transcript.update(cx, |this, cx| {
                    let row: SharedString = format!("fence-{lines}").into();
                    let ready = document(key);
                    this.highlights.cache.insert(key, ready.clone());
                    let before = crate::perf_trace::snapshot().highlight_hash_bytes;
                    for _ in 0..50 {
                        let out = this.code_highlight_for(&row, &tree, None, cx);
                        assert!(Arc::ptr_eq(out[&0].as_ref().unwrap(), &ready));
                    }
                    assert_eq!(crate::perf_trace::snapshot().highlight_hash_bytes - before, 0);
                    eprintln!("highlight evidence: lines={lines}, 50 ready requests: hashed bytes {baseline} -> 0");
                    // A queued job is also a constant-key hit, not another
                    // whole-source hash or background tokenization request.
                    this.highlights.entries.insert((row.clone(), 0), HighlightEntry {
                        key, document: None, _task: None,
                    });
                    let cache_misses = this.highlights.cache.stats().misses;
                    for _ in 0..50 {
                        assert!(this.code_highlight_for(&row, &tree, None, cx)[&0].is_none());
                    }
                    assert_eq!(this.highlights.cache.stats().misses, cache_misses);
                    assert_eq!(crate::perf_trace::snapshot().highlight_hash_bytes - before, 0);
                });
            }
        });
    }

    #[gpui::test]
    fn language_source_query_and_expired_slots_keep_their_invalidation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            let state = cx.new(|_| AppState::new());
            let transcript = cx.new(|cx| Transcript::new(state, cx));
            transcript.update(cx, |this, cx| {
                let row: SharedString = "fence".into();
                let original = DocumentHighlightKey::new(Lang::Rust, "value = 1");
                let mut query = original;
                query.query_generation += 1;
                for key in [
                    original,
                    DocumentHighlightKey::new(Lang::Rust, "value = 2"),
                    DocumentHighlightKey::new(Lang::Python, "value = 2"),
                    query,
                ] {
                    let ready = document(key);
                    this.highlights.cache.insert(key, ready.clone());
                    let actual = this
                        .highlights
                        .request(row.clone(), 0, key, "value = 2", cx)
                        .unwrap();
                    assert!(Arc::ptr_eq(&ready, &actual));
                    assert_eq!(this.highlights.entries[&(row.clone(), 0)].key, key);
                }
                let expired = document(original);
                let weak = Arc::downgrade(&expired);
                drop(expired);
                this.highlights.entries.insert(
                    (row.clone(), 0),
                    HighlightEntry {
                        key: original,
                        document: Some(weak),
                        _task: None,
                    },
                );
                let before = crate::perf_trace::snapshot().highlight_hash_bytes;
                assert!(
                    this.highlights
                        .request(row.clone(), 0, original, "value = 1", cx)
                        .is_some()
                );
                cx.set_global(Theme::light());
                assert!(
                    this.highlights
                        .request(row, 0, original, "value = 1", cx)
                        .is_some()
                );
                assert_eq!(crate::perf_trace::snapshot().highlight_hash_bytes, before);
            });
        });
    }

    #[test]
    fn incremental_prefix_and_tool_diffs_retain_prepared_keys() {
        let mut parser = IncrementalParser::default();
        parser.set_text("```rust\nlet value = 1;\n```\n\nSettled paragraph.\n\nTail paragraph.");
        let first = parser.tree().blocks[0].clone();
        let before = crate::perf_trace::snapshot().highlight_hash_bytes;
        parser.set_text("```rust\nlet value = 1;\n```\n\nSettled paragraph.\n\nTail paragraph. More.");
        assert!(Arc::ptr_eq(&first, &parser.tree().blocks[0]));
        assert_eq!(crate::perf_trace::snapshot().highlight_hash_bytes, before);
        let diff = zeron_proto::ToolDiff {
            path: "test.rs".into(),
            old_text: Some("let value = 1;\n".into()),
            new_text: "let value = 2;\n".into(),
        };
        let Some(ToolDetail::Diff {
            old_text,
            new_text,
            old_highlight_key,
            new_highlight_key,
            ..
        }) = tool_detail(None, Some(&diff), None)
        else {
            panic!()
        };
        assert_eq!(
            old_highlight_key,
            Some(DocumentHighlightKey::new(
                Lang::Rust,
                old_text.as_deref().unwrap()
            ))
        );
        assert_eq!(
            new_highlight_key,
            Some(DocumentHighlightKey::new(
                Lang::Rust,
                new_text.as_deref().unwrap()
            ))
        );
        assert_ne!(old_highlight_key, new_highlight_key);
    }
}
