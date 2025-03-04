const builtin = @import("builtin");
const std = @import("std");

const ziglua = @import("ziglua");
const Lua = ziglua.Lua;

pub fn main() !void {
    const a = std.heap.c_allocator;

    var lua = try Lua.init(a);
    defer lua.deinit();
    lua.openLibs();

    var args = if (builtin.os.tag == .windows)
        try std.process.argsWithAllocator(a)
    else
        std.process.args();
    defer args.deinit();
    var file_name: ?[:0]const u8 = null;
    {
        lua.createTable(2, 0);
        var i: switch (ziglua.lang) {
            .lua51, .lua52, .luajit => i32,
            else => ziglua.Integer,
        } = -1;
        while (args.next()) |arg| {
            if (i == 0) file_name = arg;
            _ = lua.pushString(arg);
            lua.rawSetIndex(-2, i);
            i += 1;
        }
        lua.setGlobal("arg");
    }

    lua.pushFunction(ziglua.wrap(traceback));
    switch (ziglua.lang) {
        .luajit, .lua51 => try lua.loadFile(file_name.?),
        else => try lua.loadFile(file_name.?, .binary_text),
    }
    lua.protectedCall(.{
        .results = ziglua.mult_return,
        .msg_handler = -2,
    }) catch |e| switch (e) {
        error.LuaRuntime => std.process.exit(1),
        else => return e,
    };
}

fn traceback(lua: *Lua) !i32 {
    var args = if (builtin.os.tag == .windows)
        try std.process.argsWithAllocator(lua.allocator())
    else
        std.process.args();
    defer args.deinit();
    const exe = args.next().?;
    const errmsg = try lua.toString(-1);
    (if (ziglua.lang == .lua51) tracebackLua51 else Lua.traceback)(lua, lua, errmsg, 1);
    const trace = try lua.toString(-1);
    std.debug.print("{s}: {s}\n", .{ exe, trace });
    lua.pop(1);
    return 0;
}

fn tracebackLua51(lua: *Lua, state: *Lua, msg: ?[:0]const u8, level: i32) void {
    const top = lua.getTop();
    var lv = level;
    if (msg) |s| {
        _ = lua.pushFString("%s\n", .{s.ptr});
    }
    _ = lua.pushString("stack traceback:");
    while (state.getStack(lv) catch null) |debuginfo| {
        var info = debuginfo;
        lv += 1;
        state.getInfo(.{
            .S = true, // source, short_src, first_line_defined, last_line_defined, what
            .l = true, // current_line
            .n = true, // name, name_what
        }, &info);
        _ = lua.pushFString("\n\t%s:", .{&info.short_src});
        if (info.current_line) |n| {
            _ = lua.pushFString("%d:", .{n});
        }
        _ = lua.pushString(" in ");
        switch (info.what) {
            .main => _ = lua.pushString("main chunk"),
            .lua => if (info.name_what == .other) {
                _ = lua.pushFString("function <%s:%d>", .{ &info.short_src, info.first_line_defined orelse 0 });
            } else {
                _ = lua.pushFString("function '%s'", .{info.name.?.ptr});
            },
            .c => _ = lua.pushString("?"), // TODO: show function name if it's in _G
        }
        lua.concat(lua.getTop() - top);
    }
    lua.concat(lua.getTop() - top);
}
