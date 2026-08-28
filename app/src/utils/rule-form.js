export const visibleActionTypes = [
  { value: "reject", label: "拒绝(502)" },
  { value: "redirect", label: "重定向" },
  { value: "intercept", label: "拦截确认" },
  { value: "setRequestHeader", label: "添加请求头" },
  { value: "setResponseHeader", label: "添加响应头" },
  { value: "modifyRequestBody", label: "修改请求体" },
  { value: "modifyResponseBody", label: "修改响应体" },
];

export function createDefaultAction() {
  return { type: "intercept" };
}

export function createDefaultForm() {
  return {
    name: "new rule",
    enabled: true,
    filters: [{ type: "all", value: "" }],
    actions: [createDefaultAction()],
  };
}

function normalizeTextModify(value) {
  if (typeof value === "string") {
    return { origin: "", new: value };
  }
  return {
    origin: String(value?.origin ?? ""),
    new: String(value?.new ?? ""),
  };
}

function normalizeHeaderModify(value, type) {
  return {
    type,
    key: String(value?.key ?? ""),
    value:
      value?.value == null
        ? ""
        : typeof value.value === "string"
          ? value.value
          : String(value.value.new ?? ""),
  };
}

export function parseActions(actions) {
  if (!Array.isArray(actions)) actions = [actions];
  return actions.map((action) => {
    if (typeof action === "string") {
      if (action === "logReq" || action === "logRes")
        return createDefaultAction();
      return { type: action };
    }
    if (action && typeof action === "object") {
      if (action.redirect != null) {
        return { type: "redirect", url: String(action.redirect) };
      }
      if (action.modifyRequest?.header) {
        return normalizeHeaderModify(
          action.modifyRequest.header,
          "setRequestHeader",
        );
      }
      if (action.modifyResponse?.header) {
        return normalizeHeaderModify(
          action.modifyResponse.header,
          "setResponseHeader",
        );
      }
      if (action.modifyRequest?.body) {
        return {
          type: "modifyRequestBody",
          ...normalizeTextModify(action.modifyRequest.body),
        };
      }
      if (action.modifyResponse?.body) {
        return {
          type: "modifyResponseBody",
          ...normalizeTextModify(action.modifyResponse.body),
        };
      }
    }
    return createDefaultAction();
  });
}

export function serializeAction(action) {
  switch (action.type) {
    case "reject":
      return "reject";
    case "redirect":
      return { redirect: action.url };
    case "intercept":
      return "intercept";
    case "setRequestHeader":
      return {
        modifyRequest: {
          header: {
            key: action.key,
            value: action.value,
          },
        },
      };
    case "setResponseHeader":
      return {
        modifyResponse: {
          header: {
            key: action.key,
            value: action.value,
          },
        },
      };
    case "modifyRequestBody":
      return {
        modifyRequest: {
          body: {
            origin: action.origin,
            new: action.new,
          },
        },
      };
    case "modifyResponseBody":
      return {
        modifyResponse: {
          body: {
            origin: action.origin,
            new: action.new,
          },
        },
      };
    default:
      return "intercept";
  }
}

export function serializeFormRule(form) {
  return {
    name: form.name.trim() || "new rule",
    enabled: form.enabled,
    filters: form.filters.map((filter) =>
      filter.type === "all" ? { all: null } : { [filter.type]: filter.value },
    ),
    actions: form.actions.map(serializeAction),
  };
}
