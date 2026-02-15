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

    // Create editor
    monaco.editor.create(document.getElementById("editor"), {
        value: `#include <libc/stdio.zh>
fn main() -> i32 {
    printf("Hello, Zirco!\\n");
    return 0;
}
`,
        language: "zirco",
        theme: "vs-dark",
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
        const code = monaco.editor.getModels()[0].getValue();
        const action = document.getElementById("action").value;

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
                output.textContent = `${data.stderr}${data.stdout}- Execution completed with exit code ${data.exit_code}`;
                eventSource.close();
            });
        } catch (e) {
            const output = document.getElementById("output");
            output.textContent = `Error: ${e.message}`;
        }
    };
});
