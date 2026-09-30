import Foundation

/// One rendered row of a unified diff.
struct DiffLine: Identifiable, Equatable {
    enum Kind { case context, add, remove, hunk, meta }
    let id: Int
    let kind: Kind
    let text: String
    let oldNumber: Int?
    let newNumber: Int?
}

/// The parsed section of a unified patch for one file.
struct FileDiff: Identifiable, Equatable {
    var path: String
    var oldPath: String?
    var binary = false
    var lines: [DiffLine] = []
    var additions: Int { lines.lazy.filter { $0.kind == .add }.count }
    var deletions: Int { lines.lazy.filter { $0.kind == .remove }.count }
    var id: String { path }
}

/// Pure unified-diff parsing and line diffing (no UI, no networking).
enum UnifiedDiff {
    /// Splits a `git diff` patch into per-file sections with old/new line numbers.
    static func parse(_ patch: String) -> [FileDiff] {
        var files: [FileDiff] = []
        var current: FileDiff?
        var oldLeft = 0, newLeft = 0, oldNo = 0, newNo = 0
        var inHunk: Bool { oldLeft > 0 || newLeft > 0 }

        func push(_ kind: DiffLine.Kind, _ text: String, _ old: Int?, _ new: Int?) {
            guard let count = current?.lines.count else { return }
            current?.lines.append(DiffLine(id: count, kind: kind, text: text, oldNumber: old, newNumber: new))
        }
        func flush() { if let current { files.append(current) }; current = nil }

        for raw in patch.split(separator: "\n", omittingEmptySubsequences: false) {
            var line = String(raw)
            if line.hasSuffix("\r") { line.removeLast() }
            if inHunk, current != nil {
                if line.hasPrefix("+") {
                    push(.add, String(line.dropFirst()), nil, newNo); newNo += 1; newLeft -= 1; continue
                } else if line.hasPrefix("-") {
                    push(.remove, String(line.dropFirst()), oldNo, nil); oldNo += 1; oldLeft -= 1; continue
                } else if line.hasPrefix("\\") {
                    push(.meta, line, nil, nil); continue
                } else if line.hasPrefix(" ") || line.isEmpty {
                    push(.context, String(line.dropFirst()), oldNo, newNo)
                    oldNo += 1; newNo += 1; oldLeft -= 1; newLeft -= 1; continue
                }
            }
            if line.hasPrefix("diff --git ") {
                flush()
                let paths = headerPaths(line)
                current = FileDiff(path: paths.new, oldPath: paths.old == paths.new ? nil : paths.old)
            } else if line.hasPrefix("@@"), current != nil {
                guard let h = hunkRanges(line) else { push(.meta, line, nil, nil); continue }
                oldNo = h.oldStart; newNo = h.newStart; oldLeft = h.oldCount; newLeft = h.newCount
                push(.hunk, line, nil, nil)
            } else if line.hasPrefix("+++ ") || line.hasPrefix("--- ") {
                let name = String(line.dropFirst(4))
                if current == nil { current = FileDiff(path: stripPrefix(name)) }
                if name != "/dev/null" {
                    if line.hasPrefix("+++ ") { current?.path = stripPrefix(name) }
                    else if current?.path.isEmpty == true { current?.path = stripPrefix(name) }
                } else if line.hasPrefix("+++ "), let old = current?.oldPath ?? current?.path {
                    current?.path = old // deleted file keeps its old path
                }
            } else if line.hasPrefix("rename from ") {
                current?.oldPath = String(line.dropFirst("rename from ".count))
            } else if line.hasPrefix("rename to ") {
                current?.path = String(line.dropFirst("rename to ".count))
            } else if line.hasPrefix("Binary files") || line.hasPrefix("GIT binary patch") {
                current?.binary = true
                push(.meta, line, nil, nil)
            }
            // index/mode/similarity headers carry nothing the list does not show.
        }
        flush()
        return files
    }

    /// `diff --git a/old b/new` -> (old, new). Paths with spaces resolve when both
    /// halves are identical, the common (non-rename) case.
    static func headerPaths(_ line: String) -> (old: String, new: String) {
        let body = String(line.dropFirst("diff --git ".count))
        if body.hasPrefix("a/"), let range = body.range(of: " b/") {
            if body.count % 2 == 1, body.count > 5 {
                let x = body.dropFirst(2).prefix((body.count - 5) / 2)
                if body == "a/\(x) b/\(x)" { return (String(x), String(x)) }
            }
            return (String(body[body.index(body.startIndex, offsetBy: 2)..<range.lowerBound]),
                    String(body[range.upperBound...]))
        }
        return (body, body)
    }

    private static func stripPrefix(_ name: String) -> String {
        if name.hasPrefix("a/") || name.hasPrefix("b/") { return String(name.dropFirst(2)) }
        return name
    }

    /// `@@ -a,b +c,d @@` -> starts and counts (a missing count means 1).
    static func hunkRanges(_ line: String) -> (oldStart: Int, oldCount: Int, newStart: Int, newCount: Int)? {
        let parts = line.split(separator: " ", maxSplits: 4, omittingEmptySubsequences: true)
        guard parts.count >= 3, parts[0] == "@@", parts[1].hasPrefix("-"), parts[2].hasPrefix("+") else { return nil }
        func range(_ s: Substring) -> (Int, Int)? {
            let pair = s.dropFirst().split(separator: ",", omittingEmptySubsequences: false)
            guard let start = Int(pair[0]) else { return nil }
            if pair.count > 1 { guard let count = Int(pair[1]) else { return nil }; return (start, count) }
            return (start, 1)
        }
        guard let o = range(parts[1]), let n = range(parts[2]) else { return nil }
        return (o.0, o.1, n.0, n.1)
    }

    /// Builds hunked diff lines from two file texts (used when the patch was
    /// truncated and `GetCheckoutFileDiffText` is the only source).
    static func lines(old: String?, new: String?, context: Int = 3) -> [DiffLine] {
        let a = old.map(splitLines) ?? [], b = new.map(splitLines) ?? []
        let ops = editScript(a, b)
        var rows: [DiffLine] = []
        func add(_ kind: DiffLine.Kind, _ text: String, _ o: Int?, _ n: Int?) {
            rows.append(DiffLine(id: rows.count, kind: kind, text: text, oldNumber: o, newNumber: n))
        }
        let changed = ops.indices.filter { ops[$0].kind != .context }
        guard !changed.isEmpty else { return [] }
        var keep = Set<Int>()
        for i in changed { for j in max(0, i - context)...min(ops.count - 1, i + context) { keep.insert(j) } }
        var i = 0
        while i < ops.count {
            guard keep.contains(i) else { i += 1; continue }
            var j = i
            while j < ops.count, keep.contains(j) { j += 1 }
            let slice = ops[i..<j]
            let oldStart = slice.first(where: { $0.old != nil })?.old ?? 0
            let newStart = slice.first(where: { $0.new != nil })?.new ?? 0
            let oldCount = slice.filter { $0.old != nil }.count, newCount = slice.filter { $0.new != nil }.count
            add(.hunk, "@@ -\(oldStart),\(oldCount) +\(newStart),\(newCount) @@", nil, nil)
            for op in slice { add(op.kind, op.text, op.old, op.new) }
            i = j
        }
        return rows
    }

    private static func splitLines(_ text: String) -> [String] {
        var parts = text.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        if parts.last == "" { parts.removeLast() }
        return parts
    }

    private struct Op { let kind: DiffLine.Kind; let text: String; let old: Int?; let new: Int? }

    /// LCS edit script. Inputs beyond the cell budget degrade to "all removed, all added".
    private static func editScript(_ a: [String], _ b: [String]) -> [Op] {
        var head = 0
        while head < a.count, head < b.count, a[head] == b[head] { head += 1 }
        var tail = 0
        while tail < a.count - head, tail < b.count - head, a[a.count - 1 - tail] == b[b.count - 1 - tail] { tail += 1 }
        let am = Array(a[head..<(a.count - tail)]), bm = Array(b[head..<(b.count - tail)])
        var ops: [Op] = (0..<head).map { Op(kind: .context, text: a[$0], old: $0 + 1, new: $0 + 1) }
        if am.count * bm.count > 4_000_000 {
            ops += am.enumerated().map { Op(kind: .remove, text: $1, old: head + $0 + 1, new: nil) }
            ops += bm.enumerated().map { Op(kind: .add, text: $1, old: nil, new: head + $0 + 1) }
        } else {
            let n = am.count, m = bm.count
            var table = [[UInt16]](repeating: [UInt16](repeating: 0, count: m + 1), count: n + 1)
            if n > 0, m > 0 {
                for i in stride(from: n - 1, through: 0, by: -1) {
                    for j in stride(from: m - 1, through: 0, by: -1) {
                        table[i][j] = am[i] == bm[j] ? table[i + 1][j + 1] + 1 : max(table[i + 1][j], table[i][j + 1])
                    }
                }
            }
            var i = 0, j = 0
            while i < n || j < m {
                if i < n, j < m, am[i] == bm[j] {
                    ops.append(Op(kind: .context, text: am[i], old: head + i + 1, new: head + j + 1)); i += 1; j += 1
                } else if i < n, j == m || table[i + 1][j] >= table[i][j + 1] {
                    ops.append(Op(kind: .remove, text: am[i], old: head + i + 1, new: nil)); i += 1
                } else {
                    ops.append(Op(kind: .add, text: bm[j], old: nil, new: head + j + 1)); j += 1
                }
            }
        }
        for k in 0..<tail {
            let ao = a.count - tail + k, bo = b.count - tail + k
            ops.append(Op(kind: .context, text: a[ao], old: ao + 1, new: bo + 1))
        }
        return ops
    }
}
