# EKO Prospect Agent
This is an independent Python 3.11+ package. Do not couple it to the parent image-analysis app.
Run `python -m unittest discover -s tests -v` from this directory. No network or API keys for tests.
Business invariants: extracted claims are pending; verified claims require source excerpts and a named human reviewer; event date is not retrieval date; never invent emails or buyer intent; no mail-sending transport; preserve company/email/group suppression and outreach reservations; runtime data stays ignored.
Keep score rubric/version and source provenance explainable. All schema changes require migrations. Public fixtures must be short, attributable, and contain no private customer data.
