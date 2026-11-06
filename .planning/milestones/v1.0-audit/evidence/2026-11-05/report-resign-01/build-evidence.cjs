const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const crypto = require('node:crypto');
const out = __dirname;
const phase = '.planning/phases/01-workspace-schema-lint';
const contracts = JSON.parse(fs.readFileSync(path.join(out,'contracts.json'),'utf8'));
const executed = JSON.parse(fs.readFileSync(path.join(out,'executed-tests.json'),'utf8'));
const inspected = JSON.parse(fs.readFileSync(path.join(out,'covered-file-inspection.json'),'utf8'));
const save = (name,value) => fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n');
const schema = 'crates/ontoforge-schema/';
const cli = 'crates/ontoforge/';
const expr = 'crates/ontoforge-expr/';
const test = (file,names) => names.split(',').map(name=>({file,name}));
const st = (file,names) => test(schema+'tests/'+file+'.rs',names);
const ct = (file,names) => test(cli+'tests/'+file+'.rs',names);
const et = (file,names) => test(expr+'tests/'+file+'.rs',names);
const unit = (file,names) => test(schema+'src/'+file+'.rs',names);
const load = st('load_examples','aviation_yaml_and_json_produce_identical_ir,aviation_examples_have_no_diagnostics');
const valid = ct('cli','validate_aviation_succeeds_silently');
const lintclean = [...ct('cli_lint','lint_aviation_is_clean'),...st('lint_examples','aviation_lints_clean')];
const editor = st('schema_export','editor_configs_associate_definition_schema_with_examples');
const modeline = st('schema_export','aviation_definition_documents_start_with_modeline');
const snapshot = ct('invalid_examples','invalid_examples_produce_their_code_and_a_stable_snapshot');
const evidence = [];
function add(plan, specs) {
 const truths=contracts.plans.find(item=>item.file.includes(plan+'-PLAN')).frontmatter.must_haves.truths;
 if(truths.length!==specs.length)throw new Error(plan+' expected '+truths.length+' got '+specs.length);
 specs.forEach((spec,index)=>evidence.push({id:plan+'.'+(index+1),truth:truths[index],status:'VERIFIED',...spec}));
}
function row(assertion, sources, tests=[]) {return {assertion,sources: sources.split(','),tests};}
add('01-01',[
 row('ADR-0012 records status accepted, user recommended and selected accessor semantics; chronological git evidence records ADR before implementation.', '.planning/adr/0012-definition-document-shape.md'),
 row('CLI validate calls pipeline/check_with_profile, loader reads real aviation directory and success is process exit 0.', cli+'src/lib.rs,'+cli+'src/pipeline.rs,'+schema+'src/loader/mod.rs',valid),
 row('Unknown field maps to V04 at key span; CLI asserts objects.yaml line 3 column 1 and primaryKey help.',schema+'src/loader/dispatch.rs,'+schema+'src/loader/sourcemap.rs',ct('cli','validate_unknown_field_points_at_the_misspelled_key,validate_human_output_for_unknown_field')),
 row('Serialize Diagnostic emits exactly documented fields; JSON render bypasses ANSI human renderer.',schema+'src/diag.rs,'+cli+'src/render.rs',ct('cli','json_output_has_exactly_the_documented_fields_and_no_ansi')),
 row('Missing manifest returns V14 with no location; process returns 1.',schema+'src/loader/mod.rs',ct('cli','missing_manifest_is_reported_as_v14')),
 row('Dispatch rejects unknown/reserved kinds and supplies close-name help.',schema+'src/loader/dispatch.rs',st('diagnostics','unknown_kind_is_v03_with_a_close_name,reserved_kind_is_v03_not_supported_in_v1')),
 row('Names enforce regex and case-insensitive reserved set; canonical normalizes suggestions.',schema+'src/ir/names.rs',st('diagnostics','lowercase_object_type_name_is_v07_with_canonical_help,reserved_word_is_v07_case_insensitively')),
 row('Discovery normalizes separators; SourceMap carries relative candidate name, no root join in Diagnostic file.',schema+'src/loader/discover.rs,'+schema+'src/loader/sourcemap.rs',st('loader','diagnostic_paths_use_forward_slash')),
 row('Root workspace version/edition/resolver, forbid unsafe and LF attributes remain literal declared values.','Cargo.toml,.gitattributes')
]);
add('01-02',[
 row('All ten manifests inherit workspace and current local workspace log contains passing crate tests; historical skeletons have become substantive downstream crates.','Cargo.toml,.planning/adr/0011-workspace-layout.md'),
 row('All ten path dependency manifests inspected against ADR-0011 and later approved layering; no new back-edge to CLI.','.planning/adr/0011-workspace-layout.md'),
 row('Existing local log header binds HEAD060019f; command workspace locked serial exits0; no Docker command in workspace runner.','.planning/milestones/v1.0-audit/evidence/2026-11-05/reverify-run.ps1'),
 row('Actual CI jobs run Windows/Ubuntu fmt, clippy locked and workspace test; contents read and snapshot update disabled.','.github/workflows/ci.yml'),
 row('ADR-0003 contains Spike S1 results and ADR-0012 pointer, not open spike.','.planning/adr/0003-ontology-definition-format.md'),
 row('CONTRIBUTING documents INSTA_UPDATE=always, CI no writes, invalid naming and non-reused diagnostic codes.','CONTRIBUTING.md')
]);
add('01-03',[
 row('Manifest include defaults drive candidate glob filtering and assemble merges real selected files.',schema+'src/loader/discover.rs,'+schema+'src/loader/mod.rs',st('loader','nested_files_are_merged,custom_include_restricts_files')),
 row('escapes_root rejects absolute/parent patterns; WalkDir follow_links false and filter_entry prune hidden/build directories.',schema+'src/loader/discover.rs',st('loader','include_with_parent_segment_is_rejected,absolute_include_is_rejected,dot_dirs_and_build_dirs_are_skipped,symlinked_file_is_not_followed')),
 row('is_candidate excludes manifest/config; select emits V14 on empty match.',schema+'src/loader/discover.rs',st('loader','manifest_and_lint_config_are_not_definitions')),
 row('MAX_FILE_BYTES=4MiB, read_limited checks budget and UTF8 errors become V01.',schema+'src/loader/mod.rs',st('loader','oversized_file_is_rejected,non_utf8_file_is_v01')),
 row('precheck_json uses serde_json before YAML-compatible parse; maps byte position back to chars.',schema+'src/loader/parse.rs',st('loader','json_trailing_comma_is_rejected,json_comment_is_rejected')),
 row('load_dir/load_from_memory isolated on32MiB; depth budgets reject200nested on1MiB caller stack without crashing.',schema+'src/loader/mod.rs,'+schema+'src/loader/parse.rs',st('loader','deeply_nested_yaml_is_rejected_not_crashing,deeply_nested_json_is_rejected_not_crashing')),
 row('saphyr options select strict YAML semantics and reject duplicate/merge/tag, positions preserved.',schema+'src/loader/parse.rs',st('loader','yaml_yes_no_stay_strings,duplicate_key_is_v01_with_location,merge_key_is_v01,unsupported_tag_is_v01')),
 row('SourceMap location uses shared character column conversion and original byte offset.',schema+'src/loader/sourcemap.rs,'+schema+'src/diag.rs',st('loader','bom_crlf_tab_and_cjk_locations_are_exact,json_syntax_error_column_counts_characters')),
 row('IndexMap properties preserve order; BTreeMap normalized file candidates ensure deterministic merge.',schema+'src/ir/object.rs,'+schema+'src/loader/mod.rs',st('loader','properties_keep_declared_order,file_order_does_not_change_ir')),
 row('normalize replaces backslashes and stored source path is ontology-relative.',schema+'src/loader/discover.rs,'+schema+'src/loader/sourcemap.rs',st('loader','diagnostic_paths_use_forward_slash'))
]);
add('01-04',[
 row('ObjectType carries declared optional fields; titleKey defaults to primaryKey accessor.',schema+'src/ir/object.rs',st('object_types','object_type_optional_fields_default,full_object_type_round_trips')),
 row('BaseType enum all13 plus items and required; inline properties deserialized with preserved order.',schema+'src/ir/basetype.rs,'+schema+'src/ir/object.rs',st('object_types','all_v1_base_types_parse')),
 row('array/items consistency and nearest unknown base type validated asV06.',schema+'src/validate/objects.rs,'+schema+'src/ir/basetype.rs',st('object_types','array_requires_items,items_only_for_arrays,unknown_base_type_suggests_nearest')),
 row('PK exists, required true and allowed type checks produceV10; titleKey existence checked.',schema+'src/validate/objects.rs,'+schema+'src/ir/basetype.rs',st('object_types','primary_key_must_be_required,invalid_primary_key_types_are_v10,title_key_must_exist')),
 row('ObjectTypeName typed constructor checks PascalCase and canonical help.',schema+'src/ir/names.rs',st('diagnostics','lowercase_object_type_name_is_v07_with_canonical_help')),
 row('merge refuses duplicate API name and formats first location.',schema+'src/loader/mod.rs',st('object_types','duplicate_object_type_reports_first_location')),
 row('deny_unknown_fields plus serde_path_to_error locates offending key and suggests known field.',schema+'src/loader/dispatch.rs,'+schema+'src/ir/object.rs',st('diagnostics','unknown_field_is_v04_at_the_key_with_a_suggestion')),
 row('Real YAML/JSON fixture directories are loaded; diagnostics empty and full ontology equality asserted.',schema+'src/loader/mod.rs',load)
]);
add('01-05',[
 row('Typed LinkCardinality supports three variants; ends own names and FK on to only.',schema+'src/ir/link.rs',st('link_types','parses_all_three_cardinalities,link_ir_is_typed')),
 row('ADR0012 accessor semantics match LinkEnd field description and validation namespace.','.planning/adr/0012-definition-document-shape.md,'+schema+'src/ir/link.rs,'+schema+'src/validate/links.rs',st('link_types','link_ir_is_typed,end_name_clashes_with_property_is_v08')),
 row('Referenced end objects and FK properties looked up, V09 with suggestions on miss.',schema+'src/validate/links.rs',st('link_types','unknown_end_object_type_is_v09,unknown_foreign_key_property_is_v09')),
 row('V11 enforces FK for nonM:N, no FK on M:N and matching primary type.',schema+'src/validate/links.rs',st('link_types','many_to_many_with_foreign_key_is_v11,one_to_many_without_foreign_key_is_v11,foreign_key_type_must_match_primary_key')),
 row('Namespace checks compare end names to properties and other ends; merge checks link duplicates.',schema+'src/validate/links.rs,'+schema+'src/loader/mod.rs',st('link_types','end_name_clashes_with_property_is_v08,duplicate_end_names_on_same_type_is_v08,duplicate_link_type_is_v08')),
 row('Real aviation5links span all3 cardinalities, full YAML/JSON equality asserted.', 'examples/ontologies/aviation/links.yaml',load)
]);
add('01-06',[
 row('ActionType parameters typed IndexMap with default/required/items/description and objectReference variant.',schema+'src/ir/action.rs',st('action_types','parameter_types,parameter_items_follow_the_property_rules')),
 row('Rules typed ordered Vec and ValueSource single-key deserializer; all original5 kinds and4sources checked.',schema+'src/ir/action.rs',st('action_types','parses_all_rule_kinds,parses_all_value_sources,value_source_with_two_keys_is_v06')),
 row('SubmissionCriterion stores string condition and failureMessage, actual criteria preserved across formats.',schema+'src/ir/action.rs',load),
 row('rewrite_rules dispatch rejects function and unknown rule kind withV03 and suggestions.',schema+'src/loader/dispatch.rs',st('action_types','function_rule_is_v03,unknown_rule_kind_suggests')),
 row('validate/actions resolves references and type/literal invariants before returning IR.',schema+'src/validate/actions.rs,'+schema+'src/ir/basetype.rs',st('action_types','references_must_exist,create_must_assign_primary_key_and_required,modify_cannot_assign_primary_key,current_user_only_on_string,current_time_only_on_timestamp,static_literal_must_match_property_type,parameter_default_must_be_valid_literal,numeric_values_only_widen')),
 row('Real3actions cover addLink/modifyObject/createObject and4sources; full fixture equality asserted.','examples/ontologies/aviation/actions.yaml',load)
]);
add('01-07',[
 row('analyze prechecks then spawn_scoped32MiB, program lives only inside closure and join maps panic toInternal.',expr+'src/analyze.rs',et('analyze','closure_panic_becomes_internal,flat_recursive_forms_do_not_crash_on_a_small_stack')),
 row('Lexical precheck length4096 nesting32 excludes string delimiters and rejects unterminated strings before closure.',expr+'src/limits.rs',et('analyze','too_long_expression_is_rejected_without_compiling,max_length_expression_is_accepted,brackets_inside_strings_do_not_count,triple_quoted_and_raw_strings_follow_cel_lexing,unterminated_string_is_rejected_without_compiling')),
 row('31nested compiles inside large stack,33fails before parser; caller thread explicitly1MiB and join asserted.',expr+'src/analyze.rs,'+expr+'src/limits.rs',et('analyze','deeply_nested_expression_is_rejected_not_crashing')),
 row('cel syntax error pos converted to ExprError::Syntax and test asserts1:11 for params.x+.',expr+'src/analyze.rs',et('analyze','syntax_error_has_expression_position')),
 row('walk pushes iter/accumulator scope, excludes scoped roots, handles has/select path and char coordinates.',expr+'src/walk.rs',et('analyze','comprehension_vars_are_scoped,nested_comprehension_scopes,has_macro_yields_member_path,select_chain_is_full_path,positions_are_line_and_char_column'))
]);
add('01-08',[
 row('lint_cmd returns before config/rules on validation errors; pipeline runs CEL before lint.',cli+'src/lint_cmd.rs,'+cli+'src/pipeline.rs',ct('cli_lint','lint_skips_rules_when_validation_fails,validate_codes_cannot_be_turned_off_by_config')),
 row('E08 action ordering/creation/m2m restrictions; E09 discouraged PK warning and nested-array error.',schema+'src/lint/e08.rs,'+schema+'src/lint/e09.rs',[...unit('lint/e08','triggers,clean'),...unit('lint/e09','triggers,clean,nested_array_parameter_is_an_error')]),
 row('render exit_code uses severity and deny_warnings; actual subprocess statuses checked.',cli+'src/render.rs',ct('cli_lint','lint_error_rule_exits_1,warning_only_exit_codes')),
 row('lint output shares Diagnostic and render; human snapshot and JSON noANSI checked.',cli+'src/render.rs,'+schema+'src/lint/mod.rs',ct('cli_lint','e08_human,lint_json_has_no_ansi')),
 row('load_config selects explicit config over root; rules resolve Off/Info/Warn/Error.',schema+'src/lint/config.rs,'+schema+'src/lint/mod.rs',ct('cli_lint','config_off_disables_rule,config_info_level_never_fails,config_error_level_fails,config_flag_overrides_root_file')),
 row('Parser rejects unknown IDs/levels/sections/options and returnsV15 with span/help.',schema+'src/lint/config.rs',unit('lint/config','unknown_lint_rule_id_is_error,invalid_level_is_error,unknown_section_is_error,undeclared_option_is_error')),
 row('Separate rule modules registered and trigger/clean snapshots assert output.',schema+'src/lint/e08.rs,'+schema+'src/lint/e09.rs',[...unit('lint/e08','triggers,clean'),...unit('lint/e09','triggers,clean')]),
 row('Actual fixture directories run lint with default config and no findings.',schema+'src/lint/mod.rs',lintclean)
]);
add('01-09',[
 row('Command Schema Export writes rendered schemas into chosen dir; default schemas via clap.',cli+'src/cli.rs,'+cli+'src/lib.rs,'+schema+'src/schema_export.rs',ct('cli_schema','schema_export_writes_committed_files,schema_export_creates_missing_dir,schema_export_reports_io_failure')),
 row('Schema generator uses kind discriminators and strict additionalProperties; tests inspect generated values.',schema+'src/schema_export.rs',st('schema_export','definition_schema_is_draft_2020_12,definition_variants_are_discriminated_by_kind,rule_variants_are_discriminated_by_kind')),
 row('Validator consumes exported real schemas and actual YAML docs/JSON files/manifests.',schema+'src/schema_export.rs',st('schema_export','aviation_documents_validate')),
 row('Independent invalid fixtures are rejected for unknown field/type/name by schema validator.',schema+'src/schema_export.rs',st('schema_export','invalid_examples_fail_schema')),
 row('committed schema bytes compared to current generated strings, no update env in recorded executions.',schema+'src/schema_export.rs',st('schema_export','committed_schemas_are_up_to_date,rendering_is_deterministic')),
 row('Each aviation YAML starts correct relative modeline resolving committed schema.','examples/ontologies/aviation/objects.yaml',modeline)
]);
const naming = 'crates/ontoforge-schema/src/lint/naming/';
add('01-10',[
 ...['w01','w02','w03','w04','w05'].map((name,index)=>row(['Generic/plural names and whitelist checks.','Generic property suffix collision checks.','Encoded property regex checks.','Mixed date styles reports minority style.','Bookkeeping property words checks.'][index],naming+name+'.rs',unit('lint/naming/'+name,'triggers,clean'))),
 row('registry extends naming::rules and unit registry uniqueness; all5trigger/clean snapshots active.',schema+'src/lint/mod.rs,'+naming+'mod.rs',['w01','w02','w03','w04','w05'].flatMap(name=>unit('lint/naming/'+name,'triggers,clean'))),
 row('Real aviation fixtures lint empty after all naming rules registered.',schema+'src/lint/mod.rs',lintclean)
]);
const structure = 'crates/ontoforge-schema/src/lint/structure/';
add('01-11',[
 row('W06 default60, discriminator threshold half; no dynamic null-rate query.',structure+'w06.rs',unit('lint/structure/w06','triggers,clean,max_properties_option_applies')),
 row('W07 default prefixes and4properties Jaccard>=.6 plus PK type/name guards; cluster dedup.',structure+'w07.rs',unit('lint/structure/w07','triggers,clean,many_similar_types_are_reported_once')),
 row('W08 requires3types and3same properties, reports shared set once.',structure+'w08.rs',unit('lint/structure/w08','triggers,clean,a_shared_set_is_reported_once_for_four_types')),
 row('W09 default10 and setter action single modify single property.',structure+'w09.rs',unit('lint/structure/w09','triggers,clean,max_actions_per_type_option_applies')),
 row('W10 version attrs/year suffix andW11 generic relationship names.',structure+'w10.rs,'+structure+'w11.rs',[...unit('lint/structure/w10','triggers,clean'),...unit('lint/structure/w11','triggers,clean')]),
 row('Declared option_keys validated, actual options passed to each rule and affect findings.',schema+'src/lint/config.rs,'+structure+'mod.rs',[...unit('lint/structure/w06','max_properties_option_applies,unknown_option_key_is_rejected'),...unit('lint/structure/w07','silo_prefixes_option_applies'),...unit('lint/structure/w09','max_actions_per_type_option_applies')]),
 row('Every W06-W11 has trigger/clean snapshots and all registered on real clean aviation.',schema+'src/lint/mod.rs',[...['w06','w07','w08','w09','w10','w11'].flatMap(name=>unit('lint/structure/'+name,'triggers,clean')),...lintclean]),
 row('FinalHEAD WindowsCI fmt/clippy/workspace and local workspace success are raw command evidence.','.github/workflows/ci.yml')
]);
add('01-12',[
 row('Reference checking only params/user/objects roots with typed declared paths and suggestions.',expr+'src/check.rs',et('criteria','unknown_root_variable,unknown_parameter_suggests,objects_requires_object_reference_param,unknown_object_property_suggests,user_fields_are_id_and_groups')),
 row('Scope-aware walker does not report comprehension variables.',expr+'src/walk.rs,'+expr+'src/check.rs',et('criteria','comprehension_variable_is_not_reported')),
 row('Inference bool top-level and comparison compatibility; unresolved types Any.',expr+'src/infer.rs',et('criteria','non_bool_condition,comparison_type_mismatch,unknown_types_are_any')),
 row('analyze error mapped toV13; member_paths and type_issues called inside one analyzer closure.',expr+'src/check.rs,'+expr+'src/analyze.rs',et('criteria','analyzer_errors_become_v13')),
 row('Condition SourceMap value position joined with expression internal position in message.',expr+'src/check.rs',et('criteria','diagnostic_points_at_condition_value')),
 row('Pipeline runs check_ontology and clears ontology on error; valid3aviation conditions pass.',cli+'src/pipeline.rs,'+expr+'src/check.rs',[...et('criteria','aviation_criteria_pass'),...snapshot]),
 row('ADR0006 SpikeS2 resolved records only compile, lexical guards and scoped analysis API.','.planning/adr/0006-expression-language.md')
]);
add('01-13',[
 row('Public DeTable::parse/DeValue replaces Raw/RawVisitor/private datetime constant; datetime still error.',schema+'src/lint/config.rs',unit('lint/config','datetime_and_out_of_range_integers_are_unsupported_values,toml_syntax_error_is_v15_at_its_span,config_levels_parse')),
 row('heck canonical plus nonASCII guard; explicit HTTPServer->httpServer and no misleadingUnicode help.',schema+'src/ir/names.rs',unit('ir/names','acronym_prefix_is_lowered_as_a_word,non_ascii_names_get_no_misleading_suggestion')),
 row('print_rendered calls anstream AutoStream::choice; real subprocess test checks6env modes and JSON noANSI.',cli+'src/lib.rs,'+cli+'src/render.rs',ct('cli_color','color_environment_controls_human_output_and_never_colors_json')),
 row('One offset_to_line_col definition and3real callers; chars not bytes tested independently.',schema+'src/diag.rs,'+schema+'src/loader/parse.rs,'+schema+'src/lint/config.rs,'+expr+'src/walk.rs',[...unit('diag','offset_to_line_col_takes_byte_offsets_and_counts_char_columns'),...st('loader','bom_crlf_tab_and_cjk_locations_are_exact'),...et('analyze','positions_are_line_and_char_column')]),
 row('Historical d021687..07a0e06 Cargo.lock diff changes edges only, heck0.5/anstream1 already locked; not a ban on later phases dependencies.','Cargo.lock,Cargo.toml')
]);
add('01-14',[
 {...row('Existing UAT Test2 explicit acceptance carried forward; schema assignment observed, completion popup/narrowing/bogus andJSONmanifest not individually observed. No new editor claim.','.planning/phases/01-workspace-schema-lint/01-UAT.md,.zed/settings.json,.vscode/settings.json',editor),status:'PASSED (override)'},
 row('Strict JSON two local settings, equal schema mapping and expected leading glob.','.zed/settings.json,.vscode/settings.json',editor),
 row('Manifest exclusion glob applies to every current example manifest; manifest own modeline separate.','.zed/settings.json,.vscode/settings.json',editor),
 row('All13originaldocuments carry modelines, validate/lint clean and YAMLJSON equality maintained.','examples/ontologies/aviation/objects.yaml,examples/ontologies/aviation/links.yaml,examples/ontologies/aviation/actions.yaml',[...modeline,...load,...lintclean,...valid]),
 row('Editor section states document-only modeline, project mapping,JetBrains path andCtrlSpace.','CONTRIBUTING.md'),
 row('Tests assert strict keys/local known schema, glob covers actualnonempty examples and modelinecount=docs.',schema+'tests/schema_export.rs',[...editor,...modeline])
]);
function testEvidence(spec) {
 const lines=fs.readFileSync(spec.file,'utf8').split('\n');
 const start=lines.findIndex(line=>new RegExp('\\bfn '+spec.name+'\\b').test(line));
 if(start<0)return {...spec,error:'MISSING_TEST_SOURCE'};
 let end=lines.findIndex((line,index)=>index>start&&/\bfn \w+/.test(line));
 if(end<0)end=lines.length;
 let records=executed.filter(item=>item.name===spec.name||item.name.endsWith('::'+spec.name));
 if(spec.file.includes('/src/lint/')) {const module=spec.file.split('/src/')[1].replace('.rs','').replace(/\/mod$/,'').replaceAll('/','::');records=records.filter(item=>item.name.startsWith(module+'::'));}
 if(spec.file.includes('/tests/')) {const target=path.basename(spec.file,'.rs');records=records.filter(item=>new RegExp('(?:[/\\\\]|tests[/\\\\])'+target+'[-.]').test(item.target));}
 const text=lines.slice(start,end).join('\n');
 return {...spec,line:start+1,text,assertions:text.split('\n').map((text,index)=>({line:start+index+1,text})).filter(item=>/assert|expect\(|unwrap\(|should_panic/.test(item.text)),executions:records};
}
const selected=[...new Map(evidence.flatMap(item=>item.tests).map(spec=>[spec.file+'@'+spec.name,spec])).values()].map(testEvidence);
for(const item of evidence){item.test_evidence=item.tests.map(spec=>spec.file+'@'+spec.name);item.source_assertions=item.sources.map(file=>{const bytes=fs.readFileSync(file);return {file,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),lines:bytes.toString('utf8').split('\n').length};});}
save('truth-results.json',evidence);
save('selected-test-source.json',selected);
save('acceptance-contracts.json',contracts.plans.map(plan=>({plan:plan.file,acceptance: [...plan.text.matchAll(/<acceptance_criteria>([\s\S]*?)<\/acceptance_criteria>/g)].map(match=>match[1]),verification:plan.text.match(/<verification>[\s\S]*?<\/verification>|<success_criteria>[\s\S]*?<\/success_criteria>/g),human:plan.text.match(/<human-check>[\s\S]*?<\/human-check>/g)||[]})));
const invalid=selected.filter(item=>item.error||!item.executions.length);
console.log(JSON.stringify({truths:evidence.length,verified:evidence.filter(item=>item.status==='VERIFIED').length,override:evidence.filter(item=>item.status==='PASSED (override)').length,selectedTests:selected.length,invalid:invalid.map(item=>({file:item.file,name:item.name,error:item.error})),unexercised:evidence.filter(item=>item.tests.length===0).map(item=>item.id)},null,2));

