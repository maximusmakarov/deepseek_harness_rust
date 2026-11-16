// C5.5 conformance probe: a host half that uses the `pluginManager` service the
// way a settings UI would (read the registry, toggle an entry).
export default {
  inject: ["pluginManager"],
  apply(ctx) {
    ctx.tool({
      name: "plugin_manager_probe",
      description: "Lists plugins through the pluginManager host service",
      inputSchema: { type: "object" },
      handler: async () => {
        const manager = ctx.get("pluginManager");
        if (!manager) return "{}";
        const listed = await manager.list();
        return JSON.stringify(listed);
      },
    });
    ctx.tool({
      name: "plugin_manager_toggle",
      description: "Toggles a plugin entry through the pluginManager host service",
      inputSchema: { type: "object", properties: { name: { type: "string" }, enabled: { type: "boolean" } } },
      handler: async (args) => {
        const manager = ctx.get("pluginManager");
        if (!manager) return "{}";
        return JSON.stringify(await manager.setPluginEnabled(args.name, args.enabled));
      },
    });
  },
};
