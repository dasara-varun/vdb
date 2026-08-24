# VDB Demand Validation

## What has been validated

There is evidence of adjacent demand, not proof of product-market fit. Stack Overflow’s 2025 Developer Survey collected more than 49,000 responses from 177 countries and reports broad AI-tool use, while also showing that correctness and trust remain problems: 66% of respondents were frustrated by AI results that were almost right and 45.2% said debugging AI-generated code was more time-consuming.[1]

Existing products validate the surrounding categories. Couchbase Lite provides embedded offline-first NoSQL with JSON querying, synchronization, encryption, and on-device search capabilities.[2] Oracle markets autonomous AI database capabilities for JSON/document, analytical, vector, graph, and distributed workloads.[3] These products indicate real interest but also establish strong competition.

## VDB’s narrower hypothesis

Small teams and individual developers may want a private document database that is easy to start, local by default, and able to explain operational issues without requiring a dedicated DBA. The most valuable initial capability may not be autonomous mutation. It may be verified backups, understandable health reports, schema-drift warnings, bounded query guidance, and safe recommendations.

## Target segments

| Segment | Problem to investigate | Possible paid outcome |
|---|---|---|
| Solo developers | Operational uncertainty and lack of DBA support | Local health assistant and recovery verification |
| Small SaaS teams | Backup and incident burden | Team audit, backup verification, and policy controls |
| Local-first builders | Need durable offline documents and simple sync path | Portable VDB files and later controlled replication |
| Privacy-sensitive teams | Cannot send raw data to hosted AI | Local or redacted Steward deployment |

## Interview protocol

Interview 15–20 people from the target segments. Ask them to describe their last database problem, what system they used, how they detected it, how long it took to resolve, what the failure cost, and which actions they would trust an assistant to perform. Ask to see their current workflow where appropriate, but do not request sensitive data.

Do not lead with “Would you use an AI database?” Instead, test concrete problems. Ask whether they verify restores, how they identify slow queries, whether schema changes surprise them, and what they pay today for managed database operations or observability.

## Prototype test

Show a local VDB flow that creates a collection, adds documents, detects mixed field types, explains a bounded-query warning, verifies a backup, and proposes an index or validation rule without applying it. Measure time-to-first-success, comprehension of warnings, trust in the approval boundary, repeat usage, and willingness to install the developer preview.

## Demand thresholds

Proceed beyond prototype if at least five interviewees describe the same problem without prompting, at least three install or repeatedly use the preview, and at least two agree to a paid pilot or provide a concrete procurement path. Treat “interesting” feedback without repeated use or payment as insufficient evidence.

## Pricing experiments

Test paid value around outcomes rather than storage volume. Candidate offers include verified backup and restore reports, small-team health policies, private Steward hosting, audit export, and incident diagnosis. Keep the local core free during validation so price feedback measures the value of safety and reduced operational work.

## References

[1]: https://survey.stackoverflow.co/2025/ "Stack Overflow 2025 Developer Survey"

[2]: https://www.couchbase.com/products/lite/ "Couchbase Lite - Embedded NoSQL Database for Offline-First Apps"

[3]: https://www.oracle.com/autonomous-database/ "Oracle Autonomous AI Database"
