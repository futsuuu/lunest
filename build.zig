const builtin = @import("builtin");
const std = @import("std");

const Lang = enum { lua51, lua52, lua53, lua54 };

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    for ([_]Lang{ .lua51, .lua52, .lua53, .lua54 }) |lang| {
        const ziglua = b.lazyDependency("ziglua", .{
            .target = target,
            .optimize = optimize,
            .lang = lang,
        });

        const compile_exe = b.addExecutable(.{
            .name = @tagName(lang),
            .root_source_file = b.path("lua-rt/main.zig"),
            .target = target,
            .optimize = optimize,
        });
        if (ziglua) |d| {
            compile_exe.root_module.addImport("ziglua", d.module("ziglua"));
        }
        b.installArtifact(compile_exe);

        const run_step = b.step(b.fmt("run-{s}", .{@tagName(lang)}), "Run the app");
        const run_exe = b.addRunArtifact(compile_exe);
        if (b.args) |args| {
            run_exe.addArgs(args);
        }
        run_step.dependOn(&run_exe.step);
    }
}
