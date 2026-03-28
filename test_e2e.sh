#!/bin/bash
# End-to-end test: sends memorize calls via MCP JSON-RPC (newline-delimited), waits for community summarization

echo "Sending all inserts then waiting 180s for community summarization..."
echo "Watch /tmp/lmserver.log for progress."

{
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
    sleep 2
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"memorize","arguments":{"text":"Alice is a senior software engineer who works at Acme Corp. She specializes in distributed systems and Rust programming language."}}}'
    sleep 5
    echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"memorize","arguments":{"text":"Bob is a data scientist at Acme Corp. He works closely with Alice on machine learning pipelines and data engineering."}}}'
    sleep 5
    echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"memorize","arguments":{"text":"Charlie is the CTO of Acme Corp. He manages both the engineering and data science teams."}}}'
    echo "All inserts sent, waiting for community summarization (up to 180s)..." >&2
    sleep 180
} | ./target/release/local-memory 2>/tmp/lmserver.log