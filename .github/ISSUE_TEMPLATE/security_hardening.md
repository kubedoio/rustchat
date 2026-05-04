---
name: Security Hardening
description: Propose a security improvement or report a hardening gap
labels: ["type/security"]
body:
  - type: dropdown
    id: severity
    attributes:
      label: Severity
      options:
        - informational
        - low
        - medium
        - high
        - critical
    validations:
      required: true

  - type: textarea
    id: summary
    attributes:
      label: Summary
      description: Describe the security concern or hardening opportunity.
    validations:
      required: true

  - type: textarea
    id: impact
    attributes:
      label: Impact
      description: What could go wrong if this is not addressed?
    validations:
      required: false

  - type: textarea
    id: suggestion
    attributes:
      label: Suggested mitigation
    validations:
      required: false

  - type: checkboxes
    id: disclosure
    attributes:
      label: Disclosure
      options:
        - label: This is NOT a live vulnerability (report those privately per SECURITY.md)
          required: true
