| component | usages | files | teams | with {...spread} |
|---|---|---|---|---|
| EuiSelect | 426 | 329 | 31 | 29 (6.8%) |
| EuiSuperSelect | 148 | 135 | 23 | 6 (4.1%) |
| EuiComboBox | 505 | 426 | 27 | 62 (12.3%) |
| EuiSelectable | 186 | 174 | 25 | 11 (5.9%) |

| owner | EuiSelect | EuiSuperSelect | EuiComboBox | EuiSelectable | total |
|---|---|---|---|---|---|
| @elastic/kibana-dashboards | 60 | 14 | 85 | 15 | 174 |
| @elastic/kibana-management | 74 | 7 | 34 | 28 | 143 |
| @elastic/search-ml-ux | 59 | 14 | 50 | 14 | 137 |
| @elastic/streams-ui | 33 | 20 | 42 | 18 | 113 |
| @elastic/response-ops | 40 | 7 | 25 | 8 | 80 |
| @elastic/security-investigations | 17 | 9 | 29 | 14 | 69 |
| @elastic/nightshift-sre-agent-team | 18 | 0 | 35 | 5 | 58 |
| @elastic/security-defend-workflows | 4 | 16 | 19 | 16 | 55 |
| @elastic/workchat-eng | 12 | 6 | 23 | 5 | 46 |
| @elastic/security-threat-hunting | 11 | 7 | 18 | 7 | 43 |
| @elastic/obs-signals-logs-team | 6 | 9 | 21 | 6 | 42 |
| @elastic/obs-signals-traces-team | 12 | 8 | 19 | 2 | 41 |
| @elastic/security-detection-engineering | 5 | 7 | 19 | 7 | 38 |
| @elastic/kibana-discover | 15 | 2 | 13 | 7 | 37 |
| @elastic/rna-project-team | 16 | 2 | 13 | 3 | 34 |

EuiSelect: most used props
| prop | usages |
|---|---|
| options | 419 (98.4%) |
| onChange | 403 (94.6%) |
| value | 400 (93.9%) |
| data-test-subj | 310 (72.8%) |
| aria-label | 213 (50.0%) |
| fullWidth | 186 (43.7%) |
| compressed | 133 (31.2%) |
| disabled | 99 (23.2%) |
| prepend | 61 (14.3%) |
| isInvalid | 49 (11.5%) |
| hasNoInitialSelection | 43 (10.1%) |
| id | 31 (7.3%) |
| isLoading | 29 (6.8%) |
| name | 19 (4.5%) |
| onBlur | 13 (3.1%) |

EuiSelect: most common prop sets (ignoring ^(data-test-subj|aria-.*|id|key|className|css|style|name)$)
| props | usages |
|---|---|
| onChange, options, value | 95 |
| compressed, onChange, options, value | 56 |
| fullWidth, onChange, options, value | 30 |
| compressed, fullWidth, onChange, options, value | 23 |
| fullWidth, onChange, options, prepend, value | 14 |
| disabled, fullWidth, onChange, options, prepend, value | 13 |
| compressed, disabled, onChange, options, value | 12 |
| compressed, onChange, options, prepend, value | 11 |

EuiSuperSelect: most used props
| prop | usages |
|---|---|
| valueOfSelected | 147 (99.3%) |
| options | 146 (98.6%) |
| onChange | 143 (96.6%) |
| data-test-subj | 100 (67.6%) |
| fullWidth | 90 (60.8%) |
| aria-label | 76 (51.4%) |
| disabled | 41 (27.7%) |
| compressed | 40 (27.0%) |
| prepend | 25 (16.9%) |
| isLoading | 21 (14.2%) |
| placeholder | 17 (11.5%) |
| isInvalid | 15 (10.1%) |
| style | 11 (7.4%) |
| css | 10 (6.8%) |
| popoverProps | 9 (6.1%) |

EuiSuperSelect: most common prop sets (ignoring ^(data-test-subj|aria-.*|id|key|className|css|style|name)$)
| props | usages |
|---|---|
| fullWidth, onChange, options, valueOfSelected | 27 |
| compressed, onChange, options, valueOfSelected | 18 |
| onChange, options, valueOfSelected | 16 |
| disabled, fullWidth, isLoading, onChange, options, valueOfSelected | 9 |
| disabled, fullWidth, onChange, options, valueOfSelected | 8 |
| disabled, onChange, options, valueOfSelected | 6 |
| fullWidth, onChange, options, prepend, valueOfSelected | 6 |
| onChange, options, prepend, valueOfSelected | 5 |

EuiComboBox: most used props
| prop | usages |
|---|---|
| selectedOptions | 495 (98.0%) |
| onChange | 486 (96.2%) |
| options | 460 (91.1%) |
| data-test-subj | 357 (70.7%) |
| fullWidth | 324 (64.2%) |
| singleSelection | 304 (60.2%) |
| placeholder | 301 (59.6%) |
| isClearable | 277 (54.9%) |
| aria-label | 231 (45.7%) |
| isInvalid | 181 (35.8%) |
| isDisabled | 156 (30.9%) |
| isLoading | 152 (30.1%) |
| onCreateOption | 137 (27.1%) |
| compressed | 120 (23.8%) |
| onSearchChange | 85 (16.8%) |

EuiComboBox: most common prop sets (ignoring ^(data-test-subj|aria-.*|id|key|className|css|style|name)$)
| props | usages |
|---|---|
| isClearable, onChange, options, selectedOptions, singleSelection | 21 |
| fullWidth, isClearable, onChange, options, placeholder, selectedOptions, singleSelection | 10 |
| isClearable, onChange, options, placeholder, selectedOptions, singleSelection | 10 |
| onChange, options, selectedOptions, singleSelection | 9 |
| fullWidth, isClearable, onChange, options, selectedOptions, singleSelection | 6 |
| compressed, isClearable, onChange, options, selectedOptions, singleSelection | 6 |
| fullWidth, isLoading, onChange, options, placeholder, selectedOptions, singleSelection | 6 |
| fullWidth, isClearable, isDisabled, onChange, options, placeholder, selectedOptions, singleSelection | 5 |

EuiSelectable: most used props
| prop | usages |
|---|---|
| options | 185 (99.5%) |
| onChange | 182 (97.8%) |
| listProps | 125 (67.2%) |
| searchable | 88 (47.3%) |
| singleSelection | 87 (46.8%) |
| aria-label | 86 (46.2%) |
| data-test-subj | 76 (40.9%) |
| searchProps | 70 (37.6%) |
| renderOption | 61 (32.8%) |
| emptyMessage | 43 (23.1%) |
| height | 32 (17.2%) |
| isLoading | 28 (15.1%) |
| noMatchesMessage | 28 (15.1%) |
| css | 16 (8.6%) |
| id | 12 (6.5%) |

EuiSelectable: most common prop sets (ignoring ^(data-test-subj|aria-.*|id|key|className|css|style|name)$)
| props | usages |
|---|---|
| onChange, options | 21 |
| listProps, onChange, options | 14 |
| listProps, onChange, options, singleSelection | 13 |
| onChange, options, singleSelection | 12 |
| listProps, onChange, options, renderOption, singleSelection | 6 |
| listProps, onChange, options, searchable | 6 |
| listProps, onChange, options, renderOption | 6 |
| listProps, onChange, options, searchProps, searchable, singleSelection | 5 |

Features used, by prop presence and literal values ("-" means the component doesn't offer it)
| feature | EuiSelect | EuiSuperSelect | EuiComboBox | EuiSelectable |
|---|---|---|---|---|
| multiple selection | - | - | 42.4% | 57.0% |
| search | - | - | 100.0% | 47.3% |
| async search | - | - | 16.8% | 9.1% |
| create option | - | - | 27.1% | - |
| renderOption | - | - | 10.5% | 32.8% |
| clearable | - | - | 73.7% | - |
| no initial selection | 10.1% | - | - | - |
| loading | 6.8% | 14.2% | 30.1% | 15.1% |
| invalid state | 11.5% | 10.1% | 35.8% | - |
| compressed | 31.2% | 27.0% | 23.8% | - |
| full width | 43.7% | 60.8% | 64.2% | - |
| prepend or append | 15.5% | - | 7.7% | - |

`options`: what the value is
| options | EuiSelect | EuiSuperSelect | EuiComboBox | EuiSelectable |
|---|---|---|---|---|
| call, followed | 92 | 75 | 269 | 137 |
| call | 89 | 21 | 61 | 18 |
| array literal, followed | 100 | 24 | 37 | 5 |
| parameter | 36 | 3 | 23 | 16 |
| array literal | 45 | 10 | 11 | 1 |
| absent | 7 | 2 | 45 | 1 |
| not-followed (imports) | 24 | 9 | 8 | 0 |
| one-of, followed | 8 | 3 | 15 | 0 |
| one-of | 6 | 1 | 14 | 3 |
| mutated | 11 | 0 | 11 | 1 |
| unsupported | 5 | 0 | 8 | 1 |
| parameter, followed | 2 | 0 | 0 | 3 |
| unsupported, followed | 1 | 0 | 2 | 0 |
| not-followed (imports), followed | 0 | 0 | 1 | 0 |

array literal `options`: 233; items median 2, max 14

EuiComboBox `singleSelection` values
| value | usages |
|---|---|
| object, asPlainText: true | 234 |
| literal true | 33 |
| one-of | 8 |
| call | 5 |
| not-followed (imports) | 4 |
| unsupported | 3 |
| object, asPlainText: parameter | 2 |
| object, asPlainText: false | 2 |

| pattern | usages |
|---|---|
| EuiComboBox single selection without create or async search | 203 |
| EuiComboBox single selection with asPlainText: true | 171 |
| no aria-label, aria-labelledby or id, inside an EuiFormRow | 453 |
| no aria-label, aria-labelledby or id, not inside an EuiFormRow | 13 |
| EuiSelectable inside a popover | 117 |
| props passed with {...spread} (wrappers or forwarding) | 108 |

Files that spread props into a select (wrapper candidates)
| file | usages |
|---|---|
| x-pack/platform/plugins/shared/fleet/public/applications/fleet/sections/settings/components/edit_output_flyout/index.tsx | 3 |
| x-pack/platform/plugins/shared/fleet/public/applications/fleet/sections/settings/components/edit_output_flyout/output_form_kafka_broker.tsx | 3 |
| x-pack/solutions/observability/plugins/slo/public/pages/slo_edit/components/slo_edit_form_objective_section.tsx | 3 |
| x-pack/solutions/observability/plugins/synthetics/public/apps/synthetics/components/monitor_add_edit/form/field_wrappers.tsx | 3 |
| src/platform/packages/shared/kbn-cps-utils/components/project_picker_update/blocks/frame/partials/body/filter_form/filter_input/filter_input.tsx | 2 |
| x-pack/platform/plugins/shared/agent_builder/public/application/components/tools/form/components/mcp/mcp_editable_fields.tsx | 2 |
| x-pack/platform/plugins/shared/fleet/public/applications/fleet/sections/settings/components/edit_output_flyout/output_form_kafka_compression.tsx | 2 |
| x-pack/platform/plugins/shared/osquery/public/packs/queries/ecs_mapping_editor_field.tsx | 2 |
| x-pack/solutions/observability/plugins/observability/public/components/annotations/components/forward_refs.tsx | 2 |
| x-pack/solutions/observability/plugins/slo/public/pages/slo_edit/components/indicator_section/custom_metric/metric_indicator.tsx | 2 |
| x-pack/solutions/observability/plugins/slo/public/pages/slo_edit/components/indicator_section/histogram/histogram_indicator.tsx | 2 |
| x-pack/solutions/observability/plugins/slo/public/pages/slo_edit/components/indicator_section/timeslice_metric/metric_input.tsx | 2 |
| src/platform/packages/shared/content-management/content_list/kbn-content-list-toolbar/src/filters/selectable_filter_popover.tsx | 1 |
| src/platform/packages/shared/content-management/table_list_view_table/src/components/tag_filter_panel.tsx | 1 |
| src/platform/packages/shared/context-switcher-components/src/components/selectable_list.tsx | 1 |

