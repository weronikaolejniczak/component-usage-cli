EUI usages: 87240; with css, className or style: 7291 (8.4%)

`css`: 4844 usages; what the value is
| value | usages |
|---|---|
| template, tag css | 1585 |
| object | 1117 |
| call, followed | 905 |
| template, tag css, followed | 548 |
| call | 171 |
| function, followed | 137 |
| one-of | 107 |
| array | 101 |
| object, followed | 93 |
| function | 35 |

`css`: calls it's built by, outside other calls
| callee | calls |
|---|---|
| useEuiTheme | 1844 |
| css | 264 |
| useMemoCss | 246 |
| useStyles | 208 |
| useMemo | 168 |
| useEuiFontSize | 37 |
| useListDetailPageStyles | 23 |
| logicalCSS | 19 |
| euiFullHeight | 15 |
| visorStyles | 13 |

`className`: 1541 usages; what the value is
| value | usages |
|---|---|
| literal | 1124 |
| template, tag css | 87 |
| template, tag css, followed | 76 |
| call, followed | 54 |
| call | 51 |
| one-of, followed | 36 |
| one-of | 29 |
| literal, followed | 28 |
| template | 23 |
| parameter | 21 |

`className`: calls it's built by, outside other calls
| callee | calls |
|---|---|
| useEuiTheme | 63 |
| classNames | 37 |
| cx | 20 |
| useMemo | 11 |
| classnames | 7 |
| css | 4 |
| useStyles | 4 |
| getClasses | 2 |
| actionButtonCss | 2 |
| fieldAndParamCss | 2 |

`style`: 1189 usages; what the value is
| value | usages |
|---|---|
| object | 1072 |
| object, followed | 58 |
| one-of | 23 |
| call, followed | 16 |
| one-of, followed | 14 |
| mutated | 4 |
| parameter | 1 |
| no-callers | 1 |

`style`: calls it's built by, outside other calls
| callee | calls |
|---|---|
| useEuiTheme | 183 |
| getUpdatedColumns | 24 |
| useState | 17 |
| useMemo | 12 |
| useContext | 4 |
| makeProps | 3 |
| useTimeRangeAsync | 3 |
| getAssignmentColor | 2 |
| getEmptyDroppableStyle | 2 |
| euiFontSize | 2 |

Components with at least 100 usages, by share of overridden usages
| component | usages | overridden | share | css | className | style |
|---|---|---|---|---|---|---|
| EuiModal | 224 | 68 | 30.4% | 36 | 8 | 26 |
| EuiSplitPanel.Inner | 171 | 49 | 28.7% | 41 | 5 | 6 |
| EuiBetaBadge | 111 | 31 | 27.9% | 25 | 5 | 4 |
| EuiCard | 193 | 46 | 23.8% | 34 | 10 | 4 |
| EuiPanel | 1902 | 443 | 23.3% | 295 | 90 | 73 |
| EuiNotificationBadge | 110 | 22 | 20.0% | 12 | 10 | 1 |
| EuiHealth | 259 | 50 | 19.3% | 28 | 13 | 10 |
| EuiInMemoryTable | 204 | 39 | 19.1% | 29 | 11 | 1 |
| EuiProgress | 165 | 31 | 18.8% | 18 | 6 | 13 |
| EuiAccordion | 350 | 63 | 18.0% | 47 | 20 | 5 |
| EuiPageSection | 224 | 38 | 17.0% | 22 | 0 | 16 |
| EuiDescriptionListTitle | 275 | 45 | 16.4% | 5 | 41 | 0 |
| EuiTabs | 141 | 23 | 16.3% | 19 | 2 | 4 |
| EuiDescriptionListDescription | 279 | 42 | 15.1% | 2 | 39 | 2 |
| EuiFlexGrid | 168 | 25 | 14.9% | 20 | 3 | 3 |
| EuiSuperSelect | 148 | 22 | 14.9% | 10 | 1 | 11 |
| EuiCodeBlock | 515 | 68 | 13.2% | 44 | 14 | 10 |
| EuiBadge | 1524 | 197 | 12.9% | 162 | 24 | 21 |
| EuiFlexGroup | 9141 | 1164 | 12.7% | 814 | 178 | 217 |
| EuiText | 6497 | 823 | 12.7% | 559 | 193 | 102 |

Properties set in `css`, in object keys and template text, by usages
| property | usages |
|---|---|
| width | 365 |
| height | 351 |
| display | 265 |
| padding | 258 |
| overflow | 245 |
| min-width | 214 |
| minWidth | 169 |
| font-weight | 166 |
| color | 157 |
| max-width | 144 |
| min-height | 118 |
| position | 115 |
| background-color | 106 |
| maxWidth | 97 |
| margin-left | 96 |
| border-radius | 90 |
| flex | 82 |
| margin | 75 |
| border | 71 |
| white-space | 70 |

Properties set in `style`, in object keys and template text, by usages
| property | usages |
|---|---|
| width | 214 |
| minWidth | 174 |
| maxWidth | 140 |
| height | 130 |
| minHeight | 73 |
| padding | 48 |
| fontWeight | 43 |
| marginRight | 40 |
| overflow | 36 |
| marginLeft | 32 |
| display | 32 |
| backgroundColor | 30 |
| position | 28 |
| marginTop | 28 |
| lineHeight | 23 |
| fontSize | 21 |
| marginBottom | 21 |
| paddingTop | 20 |
| alignItems | 20 |
| flexBasis | 20 |

className tokens that are EUI's own classes, in literals and template text
| class | usages |
|---|---|
| eui-textTruncate | 163 |
| eui-textBreakWord | 104 |
| eui-alignTop | 81 |
| eui-textNoWrap | 55 |
| eui-fullHeight | 30 |
| eui-fullWidth | 22 |
| eui-yScroll | 21 |
| eui-textCenter | 21 |
| eui-textLeft | 16 |
| eui-alignCenter | 11 |
| eui-displayInline | 11 |
| eui-textRight | 11 |
| eui-scrollBar | 10 |
| eui-textBreakAll | 10 |
| eui-displayInlineBlock | 10 |

styled(Eui*) wrappers: 492
| component | wrappers |
|---|---|
| EuiFlexItem | 119 |
| EuiFlexGroup | 79 |
| EuiText | 41 |
| EuiPanel | 33 |
| EuiBadge | 16 |
| EuiAccordion | 15 |
| EuiIcon | 12 |
| EuiButtonEmpty | 11 |
| EuiEmptyPrompt | 11 |
| EuiTitle | 10 |

usages through wrappers: 3355, of which through a styled(…) or euiStyled(…) call: 736; with css, className or style written by their callers: 778
| component | through wrappers | through styled(…) |
|---|---|---|
| EuiCallOut | 942 | 7 |
| EuiFlexItem | 293 | 180 |
| EuiButton | 162 | 9 |
| EuiText | 161 | 58 |
| EuiPageTemplate | 152 | 6 |
| EuiLink | 150 | 19 |
| EuiFlexGroup | 143 | 96 |
| EuiIcon | 142 | 29 |
| EuiFormRow | 100 | 12 |
| EuiPanel | 89 | 55 |

selectors targeting EUI classes in test files ((\.test\.|\.spec\.|\.stories\.|/__jest__/|/__mocks__/|/test/|/tests?_helpers?/|/cypress/|/e2e/|/ftr/|/scout/)): 1225; left out below
selectors targeting EUI classes: 1507 in 676 files; !important in those files: 350 in 118 files
| class | selectors |
|---|---|
| euiAccordion | 119 |
| euiDataGridRowCell | 93 |
| euiFlyoutBody | 83 |
| euiDataGrid | 70 |
| euiFormControlLayout | 60 |
| euiTableRow | 54 |
| euiDataGridHeaderCell | 45 |
| euiBadge | 39 |
| euiFormRow | 39 |
| euiTableRowCell | 33 |
| euiCard | 33 |
| euiFlexItem | 32 |
| euiSelectableListItem | 32 |
| euiPanel | 32 |
| euiIcon | 31 |
| euiTableCellContent | 31 |
| euiEmptyPrompt | 29 |
| euiTable | 27 |
| euiCommentEvent | 24 |
| euiDataGridRow | 22 |

| owner | overridden usages | EUI class selectors |
|---|---|---|
| @elastic/search-ml-ux | 725 | 117 |
| @elastic/kibana-dashboards | 695 | 125 |
| @elastic/kibana-management | 558 | 57 |
| @elastic/security-investigations | 522 | 179 |
| @elastic/streams-ui | 446 | 92 |
| @elastic/obs-signals-logs-team | 419 | 54 |
| @elastic/workflows-eng | 397 | 98 |
| @elastic/security-threat-hunting | 380 | 131 |
| @elastic/workchat-eng | 368 | 61 |
| @elastic/kibana-discover | 347 | 129 |
| @elastic/contextual-security-apps | 320 | 68 |
| @elastic/appex-sharedux | 311 | 69 |
| @elastic/nightshift-sre-agent-team | 277 | 20 |
| @elastic/obs-signals-traces-team | 263 | 35 |
| @elastic/response-ops | 247 | 39 |

