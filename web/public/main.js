import { AnsiUp } from "./ansi_up.js";

require.config({
    paths: {
        vs: "https://unpkg.com/monaco-editor@0.55.1/min/vs",
    },
});

require(["vs/editor/editor.main"], function () {
    monaco.languages.register({ id: "zirco" });

    monaco.languages.setMonarchTokensProvider("zirco", {
        defaultToken: "",
        tokenPostfix: ".zirco",

        keywords: [
            "if",
            "else",
            "while",
            "do",
            "for",
            "four",
            "break",
            "continue",
            "return",
            "switch",
            "match",
            "default",
            "unreachable",
            "fn",
            "let",
            "const",
            "type",
            "struct",
            "union",
            "enum",
            "as",
            "sizeof",
            "new",
        ],

        typeKeywords: [
            "i8",
            "i16",
            "i32",
            "i64",
            "isize",
            "u8",
            "u16",
            "u32",
            "u64",
            "usize",
            "bool",
        ],

        operators: [
            "->",
            "=>",
            "==",
            "!=",
            "<=",
            ">=",
            "<",
            ">",
            "&&",
            "||",
            "!",
            "<<",
            ">>",
            "&",
            "|",
            "^",
            "~",
            "+=",
            "-=",
            "*=",
            "/=",
            "%=",
            "&=",
            "|=",
            "^=",
            "<<=",
            ">>=",
            "++",
            "--",
            "+",
            "-",
            "*",
            "/",
            "%",
            "=",
            ".",
            "::",
            "?",
            ";",
        ],

        symbols: /[=><!~?:&|+\-*\/\^%\.]+/,

        tokenizer: {
            root: [
                // comments
                [/\/\/.*$/, "comment"],

                // preprocessor
                [/^\s*#\s*include\b/, "keyword.directive"],
                [/^\s*#\s*pragma\s+once\b/, "keyword.directive"],

                // function declaration
                [
                    /\b(fn)(\s+)([a-zA-Z_]\w*)/,
                    ["keyword", "", "entity.name.function"],
                ],

                // type declaration
                [/\btype\s+([a-zA-Z_]\w*)/, ["keyword", "entity.name.type"]],

                // struct / union / enum
                [
                    /\b(struct|union|enum)\s+([a-zA-Z_]\w*)?/,
                    ["keyword", "entity.name.type"],
                ],

                // numbers
                [/\b0x[0-9a-fA-F_]+\b/, "number.hex"],
                [/\b0b[01_]+\b/, "number.binary"],
                [/\b\d[\d_]*\b/, "number"],

                // strings
                [
                    /"/,
                    {
                        token: "string.quote",
                        bracket: "@open",
                        next: "@string",
                    },
                ],

                // chars
                [
                    /'/,
                    {
                        token: "string.quote",
                        bracket: "@open",
                        next: "@char",
                    },
                ],

                // identifiers
                [
                    /[a-zA-Z_]\w*/,
                    {
                        cases: {
                            "@keywords": "keyword",
                            "@typeKeywords": "type",
                            "@default": "identifier",
                        },
                    },
                ],

                // operators
                [
                    /@symbols/,
                    {
                        cases: {
                            "@operators": "operator",
                            "@default": "",
                        },
                    },
                ],

                // delimiters
                [/[{}()\[\]]/, "@brackets"],
                [/[;,]/, "delimiter"],
            ],

            string: [
                [/[^\\"]+/, "string"],
                [/\\(n|r|t|\\|"|')/, "string.escape"],
                [/\\x[0-9a-fA-F]{2}/, "string.escape"],
                [/\\u\{[0-9a-fA-F]+\}/, "string.escape"],
                [/\\./, "string.escape.invalid"],
                [
                    /"/,
                    {
                        token: "string.quote",
                        bracket: "@close",
                        next: "@pop",
                    },
                ],
            ],

            char: [
                [/[^\\']+/, "string"],
                [/\\(n|r|t|\\|"|')/, "string.escape"],
                [/\\x[0-9a-fA-F]{2}/, "string.escape"],
                [/\\u\{[0-9a-fA-F]+\}/, "string.escape"],
                [/\\./, "string.escape.invalid"],
                [
                    /'/,
                    {
                        token: "string.quote",
                        bracket: "@close",
                        next: "@pop",
                    },
                ],
            ],
        },
    });

    const defaultCode = `#include <libc/stdio.zh>
fn main() -> i32 {
    printf("Hello, Zirco!\\n");
    return 0;
}
`;

    const MAX_URL_CODE_LENGTH = 50000;

    function loadFromURL() {
        try {
            const params = new URLSearchParams(location.hash.slice(1));
            const code = params.get("code");
            const action = params.get("action");
            return { code, action };
        } catch (e) {
            return { code: null, action: null };
        }
    }

    function updateURL(code, action) {
        if (code.length > MAX_URL_CODE_LENGTH) {
            return;
        }
        const params = new URLSearchParams();
        params.set("code", code);
        params.set("action", action);
        history.replaceState(null, "", "#" + params.toString());
    }

    const { code: urlCode, action: urlAction } = loadFromURL();

    const actionSelect = document.getElementById("action");
    if (urlAction) {
        const matchingOption = Array.from(actionSelect.options).find(
            (opt) => opt.value === urlAction,
        );
        if (matchingOption) {
            actionSelect.value = urlAction;
        }
    }

    const editorValue =
        urlCode !== null && urlCode.length <= MAX_URL_CODE_LENGTH
            ? urlCode
            : defaultCode;

    const editor = monaco.editor.create(document.getElementById("editor"), {
        value: editorValue,
        language: "zirco",
        theme: "vs-dark",
    });

    if (urlCode === null) {
        updateURL(editor.getValue(), actionSelect.value);
    }

    let urlUpdateTimer = null;
    editor.onDidChangeModelContent(() => {
        clearTimeout(urlUpdateTimer);
        urlUpdateTimer = setTimeout(() => {
            updateURL(editor.getValue(), actionSelect.value);
        }, 300);
    });

    actionSelect.addEventListener("change", () => {
        updateURL(editor.getValue(), actionSelect.value);
    });

    const ver = document.getElementById("toolchain");
    fetch("https://play.zirco.dev/api/v1/version")
        .then((res) => res.json())
        .then((data) => {
            ver.textContent = data.version;
        })
        .catch((e) => {
            console.error("Failed to fetch version:", e);
            ver.textContent = "unknown";
        });

    document.getElementById("run").onclick = async function run() {
        const code = editor.getValue();
        const action = actionSelect.value;

        const { jobId } = await fetch("https://play.zirco.dev/api/v1/execute", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                code,
                task: action,
            }),
        }).then((res) => {
            if (!res.ok) {
                const output = document.getElementById("output");
                output.textContent = `Error: ${res.status} ${res.statusText}`;
                throw new Error(`HTTP error! status: ${res.status}`);
            }
            return res.json();
        });

        // Listen for SSE
        try {
            const output = document.getElementById("output");
            output.textContent = "Running...\n";
            const eventSource = new EventSource(
                `https://play.zirco.dev/api/v1/stream/${jobId}`,
            );
            eventSource.addEventListener("timeout", (event) => {
                output.textContent += "\nExecution timed out.";
                eventSource.close();
            });
            eventSource.addEventListener("complete", (event) => {
                const data = JSON.parse(event.data);
                let text = `${data.stderr}${data.stdout}- Execution completed with exit code ${data.exit_code}`;
                let ansi = new AnsiUp();
                text = ansi.ansi_to_html(text);
                // safe because ansiup sanitizes the output
                output.innerHTML = text;
                eventSource.close();
            });
        } catch (e) {
            const output = document.getElementById("output");
            output.textContent = `Error: ${e.message}`;
        }
    };
});
