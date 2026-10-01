import Foundation

/// One project's slice of the home list. `space == nil` is the "No project"
/// home group (sessions started without a repository).
struct CompanionProjectGroup: Identifiable {
    static let homeID = "home"
    let id: String
    let space: HostSpace?
    let name: String
    let chats: [HostChat]
    let needsYou: Int
    let running: Int

    var isHome: Bool { space == nil }
    /// The project filter / new-session value: empty means "no project".
    var projectID: String { space?.id ?? "" }
    /// Filtering must distinguish the home group from the All-projects value.
    var filterID: String { space?.id ?? CompanionModel.noProjectFilter }
    /// Mono path hint, home-relative where the host path allows it.
    var hint: String { space.map { CompanionProjectGroup.pathHint($0.path) } ?? "~" }

    /// `~/code/noches` style: the last two components, never the full path.
    static func pathHint(_ path: String) -> String {
        let parts = path.split(separator: "/").map(String.init)
        guard parts.count > 2 else { return path }
        return "…/" + parts.suffix(2).joined(separator: "/")
    }
}

enum CompanionGrouping: String, CaseIterable {
    case status, project
    var title: String { self == .status ? "By status" : "By project" }
}

extension CompanionModel {
    static let noProjectFilter = "__no_project__"
    /// Groups `chats` by project. Chats whose project is unknown fall into the
    /// home group. Order: groups with the most urgent session first, then the
    /// most recent activity, then name; empty projects (when `keepEmpty`) sort
    /// last, alphabetically. Chat order inside a group is preserved.
    static func projectGroups(chats: [HostChat], spaces: [HostSpace], keepEmpty: Bool,
                              state: (HostChat) -> SessionState) -> [CompanionProjectGroup] {
        let known = Set(spaces.map(\.id))
        var buckets: [String: [HostChat]] = [:]
        for chat in chats {
            let key = chat.spaceId.flatMap { known.contains($0) ? $0 : nil } ?? CompanionProjectGroup.homeID
            buckets[key, default: []].append(chat)
        }
        func make(_ id: String, space: HostSpace?, name: String) -> CompanionProjectGroup {
            let items = buckets[id] ?? []
            return CompanionProjectGroup(id: id, space: space, name: name, chats: items,
                                         needsYou: items.filter { state($0).needsYou }.count,
                                         running: items.filter { state($0).running }.count)
        }
        var groups = spaces.compactMap { space -> CompanionProjectGroup? in
            guard keepEmpty || buckets[space.id] != nil else { return nil }
            return make(space.id, space: space, name: space.displayName)
        }
        if buckets[CompanionProjectGroup.homeID] != nil { groups.append(make(CompanionProjectGroup.homeID, space: nil, name: "No project")) }
        func rank(_ group: CompanionProjectGroup) -> Int { group.chats.map { state($0).attentionRank }.min() ?? Int.max }
        func latest(_ group: CompanionProjectGroup) -> Date { group.chats.map(\.activityDate).max() ?? .distantPast }
        return groups.sorted { a, b in
            if a.chats.isEmpty != b.chats.isEmpty { return !a.chats.isEmpty }
            if rank(a) != rank(b) { return rank(a) < rank(b) }
            if latest(a) != latest(b) { return latest(a) > latest(b) }
            return a.name.localizedCaseInsensitiveCompare(b.name) == .orderedAscending
        }
    }

    func projectGroups(chats: [HostChat], keepEmpty: Bool = false) -> [CompanionProjectGroup] {
        Self.projectGroups(chats: chats, spaces: localSpaces, keepEmpty: keepEmpty) { self.state($0) }
    }
}

/// Collapsed project ids persisted as one newline-joined `@AppStorage` string.
enum CompanionCollapsed {
    static func ids(_ raw: String) -> Set<String> { Set(raw.split(separator: "\n").map(String.init)) }
    static func toggled(_ raw: String, id: String) -> String {
        var set = ids(raw)
        if set.contains(id) { set.remove(id) } else { set.insert(id) }
        return set.sorted().joined(separator: "\n")
    }
}
