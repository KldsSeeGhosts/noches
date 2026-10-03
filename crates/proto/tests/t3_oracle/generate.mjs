#!/usr/bin/env node
// Mechanical Rust translation of the extracted Effect schema AST. No engine behavior.
import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../../..");
const fixture = (n) => JSON.parse(readFileSync(`${here}/fixtures/${n}.json`));
const { definitions: defs, origins } = fixture("contracts");
const tools = fixture("tools");
const hash = (v) => createHash("sha256").update(JSON.stringify(v)).digest("hex");
const ident = (s) => {
  const v = String(s).replace(/([a-z0-9])([A-Z])/g, "$1_$2").replace(/[^a-zA-Z0-9_]/g, "_").toLowerCase();
  return ["type","ref","self","match","use","mod","loop","async","move","in","default","enum","struct","where"].includes(v) ? `r#${v}` : v;
};
const pascal = (s) => {
  const v = String(s).split(/[^a-zA-Z0-9]+/).filter(Boolean).map((x) => x[0].toUpperCase()+x.slice(1)).join("");
  return /^[0-9]/.test(v) ? `V${v}` : v || "Empty";
};
const moduleOf = (n) => n==="OrchestratorMcpProviderCapability" || ["providerInstance.ts", "model.ts", "modelSelection.ts"].includes(origins[n])
  ? "provider_instance" : origins[n]?.startsWith("toolkit:") || origins[n]==="effect-framework" || ["orchestratorMcp.ts","worktreeMcp.ts","threadMetadataMcp.ts","previewAutomation.ts"].includes(origins[n])
    ? "orchestration_mcp" : "orchestration";
const output = Object.fromEntries(["orchestration","orchestration_mcp","provider_instance"].map((m) => [m, []]));
const emitted = new Map();
const codecOverrides = new Set(["ModelSelection","ProviderOptionSelections","OrchestratorMcpTargetOptions","OrchestratorMcpScheduleTaskInput","ScheduledTaskSchedule","ScheduledTaskUpsertSchedule","OrchestrationV2NotificationSource","OrchestrationV2PendingBackgroundTask"]);
const header = (m) => `//! Generated T3 V2 wire contracts. See docs/orchestration/contracts.md.\n//! Regenerate: node crates/proto/tests/t3_oracle/generate.mjs\n//! Source: T3 Tools Inc., MIT, pinned in tests/t3_oracle/fixtures/provenance.json.\n#![allow(unused_imports)]\nuse serde::{Deserialize, Serialize};\nuse std::collections::BTreeMap;\n${["orchestration","orchestration_mcp","provider_instance"].filter((n)=>n!==m).map((n)=>`use crate::${n}::*;`).join("\n")}\n`;
const derives = "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]";
function flatten(s, expandRefs = false, seen = new Set()) {
  if (s.kind === "union") return s.members.flatMap((m) => flatten(m, expandRefs, seen));
  if (expandRefs && s.kind === "ref" && !seen.has(s.name)) return flatten(defs[s.name], true, new Set([...seen,s.name]));
  return [s];
}
const unique = (a) => [...new Map(a.map((x) => [JSON.stringify(x),x])).values()];
function simplify(s) {
  if (s.kind !== "union") return s;
  const members = unique(flatten(s).filter((m) => m.kind !== "undefined"));
  return members.length === 1 ? simplify(members[0]) : {kind:"union",members};
}
function rust(s, name, module) {
  s = simplify(s);
  switch(s.kind) {
    case "ref": return s.name;
    case "string": return "String";
    case "number": return s.integer ? "i64" : "JsonNumber";
    case "boolean": return "bool";
    case "null": case "unknown": return "serde_json::Value";
    case "never": return "Never";
    case "array": return `Vec<${rust(s.element,name+"Item",module)}>`;
    case "tuple": return `(${s.elements.map((e,i)=>rust(e,name+`Item${i+1}`,module)).join(", ")}${s.elements.length===1 ? "," : ""})`;
    case "record": return `BTreeMap<${s.key ? rust(s.key,name+"Key",module) : "String"}, ${rust(s.value,name+"Value",module)}>`;
    case "union": {
      const nonnull = s.members.filter((m) => m.kind !== "null");
      if (nonnull.length !== s.members.length) return `Option<${rust(simplify({kind:"union",members:nonnull}),name,module)}>`;
      break;
    }
  }
  emit(name,s,module);
  return name;
}
function fields(s, name, module, omit = new Set()) {
  return s.fields.filter((f)=>!omit.has(f.name)).map((f)=>{
    const ty = rust(f.shape,name+pascal(f.name),module);
    const attrs = [`rename = ${JSON.stringify(f.name)}`];
    const hasDefault = Object.hasOwn(f,"default");
    if (hasDefault) {
      const fun = `${ident(name)}_${ident(f.name).replace("r#","")}_default`;
      output[module].push(`fn ${fun}() -> ${ty} { serde_json::from_str(${JSON.stringify(JSON.stringify(f.default))}).expect("upstream default") }\n`);
      attrs.push(`default = "${fun}"`);
    } else if (f.optional) attrs.push('default','skip_serializing_if = "Optional::is_absent"');
    else if (ty.startsWith("Option<")) attrs.push('deserialize_with = "required_nullable"');
    return `    #[serde(${attrs.join(", ")})]\n    pub ${ident(f.name)}: ${f.optional && !hasDefault ? `Optional<${ty}>` : ty},`;
  }).join("\n");
}
function emit(name,s,module=moduleOf(name)) {
  if (emitted.has(name)) {
    if (JSON.stringify(emitted.get(name)) !== JSON.stringify(s)) throw Error(`Type collision: ${name}`);
    return;
  }
  emitted.set(name,s);
  s=simplify(s);
  let text;
  const isId = (name.endsWith("Id") || name === "CheckpointRef" || name === "ProviderDriverKind") && s.kind === "string";
  if (isId) {
    text = `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\n#[serde(transparent)]\npub struct ${name}(pub String);\nimpl From<String> for ${name} { fn from(value: String) -> Self { Self(value) } }\nimpl From<&str> for ${name} { fn from(value: &str) -> Self { Self(value.to_owned()) } }\nimpl AsRef<str> for ${name} { fn as_ref(&self) -> &str { &self.0 } }\nimpl std::fmt::Display for ${name} { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { self.0.fmt(f) } }\n`;
  } else if (s.kind === "object") {
    text = `${derives}\npub struct ${name} {\n${fields(s,name,module)}\n}\n`;
  } else if (s.kind === "literal" && typeof s.value !== "string") {
    const expected = JSON.stringify(s.value);
    text = `#[derive(Debug, Clone, PartialEq)]\npub struct ${name};\nimpl Serialize for ${name} { fn serialize<S: serde::Serializer>(&self,serializer:S)->Result<S::Ok,S::Error> { serde_json::json!(${expected}).serialize(serializer) } }\nimpl<'de> Deserialize<'de> for ${name} { fn deserialize<D:serde::Deserializer<'de>>(deserializer:D)->Result<Self,D::Error> { let value=serde_json::Value::deserialize(deserializer)?; if value==serde_json::json!(${expected}) { Ok(Self) } else { Err(serde::de::Error::custom(${JSON.stringify("expected literal "+expected)})) } } }\n`;
  } else if (s.kind === "literal" || (s.kind==="union" && unique(flatten(s,true)).every((m)=>m.kind==="literal" && typeof m.value==="string"))) {
    const members = unique(flatten(s,true));
    text = `${derives}\npub enum ${name} {\n${members.map((m)=>`    #[serde(rename = ${JSON.stringify(m.value)})]\n    ${pascal(m.value)},`).join("\n")}\n}\n`;
  } else if (s.kind === "union") {
    const expanded = unique(flatten(s,true));
    const tag = ["type","kind","_tag","strategy","operation"].find((tag)=>expanded.every((m)=>m.kind==="object" && m.fields.some((f)=>f.name===tag && !f.optional && !Object.hasOwn(f,"default") && unique(flatten(f.shape,true)).every((x)=>x.kind==="literal"))));
    if (tag) {
      const variants = new Map();
      for (const member of expanded) {
        for (const literal of unique(flatten(member.fields.find((f)=>f.name===tag).shape,true))) {
          if (variants.has(literal.value)) continue;
          variants.set(literal.value,member);
        }
      }
      text = `${derives}\n#[serde(tag = ${JSON.stringify(tag)})]\npub enum ${name} {\n${[...variants].map(([value,member])=>{
        const payload = name+pascal(value);
        emit(payload,{...member,fields:member.fields.filter((f)=>f.name!==tag)},module);
        return `    #[serde(rename = ${JSON.stringify(value)})]\n    ${pascal(value)}(Box<${payload}>),`;
      }).join("\n")}\n}\n`;
    } else {
      const members = unique(flatten(s));
      text = `${derives}\n#[serde(untagged)]\npub enum ${name} {\n${members.map((m,i)=>{
        const child=name+`Variant${i+1}`;
        const ty=rust(m,child,module);
        return `    Variant${i+1}(${name==="ChatAttachment" ? `Box<${ty}>` : ty}),`;
      }).join("\n")}\n}\n`;
    }
  } else {
    const ty = rust(s,name+"Value",module);
    text=`pub type ${name} = ${ty};\n`;
  }
  if (codecOverrides.has(name)) {
    // Keep the public typed shape, but normalize using the upstream compatibility
    // codec before deserializing the otherwise mechanically derived mirror.
    if (text.startsWith("pub type")) {
      const ty=rust(s,name+"Value",module);
      text=`${derives}\n#[serde(try_from = "serde_json::Value")]\npub struct ${name}(pub ${ty});\nimpl TryFrom<serde_json::Value> for ${name} { type Error=String; fn try_from(value:serde_json::Value)->Result<Self,String> { serde_json::from_value(normalize_contract(${JSON.stringify(name)},value)?).map(Self).map_err(|e|e.to_string()) } }\n`;
      // Serialize wrapper arrays transparently.
      text=text.replace('Serialize, Deserialize','Deserialize');
      text+=`impl Serialize for ${name} { fn serialize<S:serde::Serializer>(&self,serializer:S)->Result<S::Ok,S::Error> { self.0.serialize(serializer) } }\n`;
    } else {
      const mirror=name+"Canonical";
      const mirrorText=text.replace(`pub struct ${name} {`,`struct ${mirror} {`).replace(`pub enum ${name} {`,`enum ${mirror} {`);
      output[module].push(mirrorText);
      text=text.replace(derives,`${derives}\n#[serde(try_from = "serde_json::Value"${name==="OrchestrationV2NotificationSource" ? ', into = "serde_json::Value"' : ""})]`);
      const names=s.kind==="object" ? s.fields.map((f)=>ident(f.name)) : null;
      const variants=s.kind==="union" ? unique(flatten(s,true)).filter((m)=>m.kind==="object").flatMap((m)=>unique(flatten(m.fields.find((f)=>f.name==="kind"||f.name==="type").shape,true)).map((l)=>pascal(l.value))) : null;
      const arms=variants ? [...new Set(variants)].map((v)=>`${mirror}::${v}(value)=>Self::${v}(value)`).join(",") : "";
      text+=`impl TryFrom<serde_json::Value> for ${name} { type Error=String; fn try_from(value:serde_json::Value)->Result<Self,String> { let value: ${mirror}=serde_json::from_value(normalize_contract(${JSON.stringify(name)},value)?).map_err(|e|e.to_string())?; Ok(${names ? `Self{${names.map((n)=>`${n}:value.${n}`).join(",")}}` : `match value {${arms}}`}) } }\n`;
      if(name==="OrchestrationV2NotificationSource") {
        const reverse=[...new Set(variants)].map((v)=>`${name}::${v}(value)=>${mirror}::${v}(value)`).join(",");
        text+=`impl From<${name}> for serde_json::Value { fn from(value:${name})->Self { let canonical=match value {${reverse}}; encode_notification_source(serde_json::to_value(canonical).expect("notification encoding")) } }\n`;
      }
    }
  }
  if (["RuntimeMode","ProviderInteractionMode","ProviderApprovalDecision","ProviderApprovalOption"].includes(name))
    text="// TODO(merge): move to runtime_policy; re-export here after orch/runtime-policy lands.\n"+text;
  output[module].push(text);
}
for(const name of Object.keys(defs).sort()) emit(name,defs[name]);
const checks = [];
const samples = {};
function sample(s, depth=0, full=false) {
  s=simplify(s);
  if (depth>25) return null;
  switch(s.kind) {
    case "ref": return sample(defs[s.name],depth+1,full);
    case "string": return "fixture";
    case "number": return s.integer ? 1 : 1.5;
    case "boolean": return false;
    case "literal": return s.value;
    case "array": return [];
    case "tuple": return s.elements.map((e)=>sample(e,depth+1,full));
    case "record": return {};
    case "null": case "unknown": return null;
    case "object": return Object.fromEntries(s.fields.filter((f)=>full || !f.optional || Object.hasOwn(f,"default")).map((f)=>{
      let value=Object.hasOwn(f,"default") ? f.default : sample(f.shape,depth+1,full);
      if(f.name==="everyMs") value=60000;
      if(f.name==="timeOfDay") value="09:00";
      if(f.name.endsWith("At") && value==="fixture") value="2026-10-03T12:00:00.000Z";
      return [f.name,value];
    }));
    case "union": return sample(s.members.find((m)=>m.kind==="null")??s.members[0],depth+1,full);
    case "never": return null;
    default: throw Error(`sample ${s.kind}`);
  }
}
for(const [name,s] of Object.entries(defs)) {
  if (s.kind==="never") continue;
  const variants = s.kind==="union" ? unique(flatten(s,true)) : [s];
  samples[name] = unique(variants.flatMap((m)=>[sample(m),sample(m,0,true)]));
  if(name==="OrchestrationV2NotificationSource") samples[name]=unique(samples[name].map((v)=>{
    if(v.kind==="subagent") return {...v,kind:"background_task",work:"subagent"};
    if(v.kind==="command") return {...v,kind:"background_command"};
    return v;
  }));
  checks.push(`        ${JSON.stringify(name)} => round_trip::<crate::${moduleOf(name)}::${name}>(value),`);
}
writeFileSync(`${here}/fixtures/serde-cases.json`,JSON.stringify(samples,null,2)+"\n");
output.orchestration_mcp.push(`/// Pinned descriptor digest: includes schemas, defaults, annotations and error tags.\npub const TOOL_CONTRACT_DIGESTS: &[(&str, &str)] = &[\n${tools.map((t)=>`    (${JSON.stringify(t.name)}, ${JSON.stringify(hash(t))}),`).join("\n")}\n];\npub const DOMAIN_CONTRACT_DIGESTS: &[(&str, &str)] = &[\n${Object.entries(defs).map(([n,s])=>`    (${JSON.stringify(n)}, ${JSON.stringify(hash(s))}),`).join("\n")}\n];\n`);
output.orchestration_mcp.push(`${derives}\n#[serde(tag="name",content="arguments")]\npub enum OrchestrationToolInput {\n${tools.map((t)=>`    #[serde(rename=${JSON.stringify(t.name)})]\n    ${pascal(t.name)}(Box<${t.rust.input}>),`).join("\n")}\n}\n`);
output.orchestration_mcp.push(readFileSync(`${here}/mcp-support.rs.txt`,"utf8"));
output.orchestration.push(readFileSync(`${here}/support.rs.txt`,"utf8"));
for(const [module,parts] of Object.entries(output)) writeFileSync(`${root}/crates/proto/src/${module}.rs`,header(module)+parts.join("\n"));
writeFileSync(`${here}/round_trips.rs`, `// Generated fixture dispatch; see generate.mjs.\nfn check_case(name: &str, value: &serde_json::Value) {\n    match name {\n${checks.join("\n")}\n        _ => panic!("unmapped oracle contract: {name}"),\n    }\n}\n`);
execFileSync(process.env.RUSTFMT ?? "rustfmt",["--edition","2024",...Object.keys(output).map((m)=>`${root}/crates/proto/src/${m}.rs`),`${here}/round_trips.rs`],{stdio:"inherit"});
const generatedPaths=[...Object.keys(output).map((m)=>`crates/proto/src/${m}.rs`),"crates/proto/tests/t3_oracle/round_trips.rs"];
writeFileSync(`${here}/fixtures/generated-artifacts.json`,JSON.stringify(Object.fromEntries(generatedPaths.map((p)=>[p,createHash("sha256").update(readFileSync(`${root}/${p}`)).digest("hex")]).sort()),null,2)+"\n");
console.log(`Generated ${emitted.size} Rust types and ${checks.length} fixture dispatches`);
