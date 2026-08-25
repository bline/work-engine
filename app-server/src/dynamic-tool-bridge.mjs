function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
}

function key(namespace, name) {
  return `${namespace ?? ""}\u0000${name}`;
}

function normalizeResult(value) {
  if (value?.contentItems && typeof value.success === "boolean") return value;
  const text = typeof value === "string" ? value : JSON.stringify(value ?? null);
  return { success: true, contentItems: [{ type: "inputText", text }] };
}

export class DynamicToolBridge {
  constructor(registrations = []) {
    this.registrations = new Map();
    for (const registration of registrations) {
      requireText(registration?.name, "dynamic tool name");
      requireText(registration?.description, "dynamic tool description");
      if (typeof registration.handler !== "function") {
        throw new TypeError(`dynamic tool ${registration.name} requires a handler`);
      }
      if (!registration.inputSchema || typeof registration.inputSchema !== "object") {
        throw new TypeError(`dynamic tool ${registration.name} requires an input schema`);
      }
      if (registration.namespace != null) requireText(registration.namespace, "namespace");
      const registrationKey = key(registration.namespace ?? null, registration.name);
      if (this.registrations.has(registrationKey)) {
        throw new Error(`duplicate dynamic tool: ${registration.name}`);
      }
      this.registrations.set(registrationKey, {
        namespace: registration.namespace ?? null,
        name: registration.name,
        description: registration.description,
        inputSchema: registration.inputSchema,
        deferLoading: registration.deferLoading === true,
        handler: registration.handler,
      });
    }
  }

  specs() {
    const direct = [];
    const namespaces = new Map();
    for (const registration of this.registrations.values()) {
      const specification = {
        type: "function",
        name: registration.name,
        description: registration.description,
        inputSchema: registration.inputSchema,
        ...(registration.deferLoading ? { deferLoading: true } : {}),
      };
      if (registration.namespace == null) direct.push(specification);
      else {
        const tools = namespaces.get(registration.namespace) ?? [];
        tools.push(specification);
        namespaces.set(registration.namespace, tools);
      }
    }
    return [
      ...direct,
      ...[...namespaces.entries()].map(([name, tools]) => ({
        type: "namespace",
        name,
        description: `Work Engine dynamic tools for ${name}`,
        tools,
      })),
    ];
  }

  async dispatch(params) {
    const registration = this.registrations.get(key(params?.namespace ?? null, params?.tool));
    if (!registration) {
      return {
        success: false,
        contentItems: [{
          type: "inputText",
          text: `No thread-scoped dynamic tool is bound for ${params?.tool ?? "unknown"}`,
        }],
      };
    }
    try {
      return normalizeResult(await registration.handler(params.arguments, params));
    } catch (error) {
      return {
        success: false,
        contentItems: [{
          type: "inputText",
          text: error instanceof Error ? error.message : "dynamic tool failed",
        }],
      };
    }
  }
}
