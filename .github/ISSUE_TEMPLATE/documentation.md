---
name: Documentation
description: Report missing, incorrect, or unclear documentation
labels: ["type/documentation"]
body:
  - type: dropdown
    id: area
    attributes:
      label: Area
      options:
        - user guide
        - admin guide
        - developer guide
        - architecture docs
        - API docs
        - README / root docs
    validations:
      required: true

  - type: textarea
    id: problem
    attributes:
      label: What is missing or unclear?
    validations:
      required: true

  - type: textarea
    id: suggestion
    attributes:
      label: Suggested improvement
      placeholder: |
        - Add section about ...
        - Clarify ...
        - Fix incorrect command ...
    validations:
      required: false

  - type: checkboxes
    id: willing
    attributes:
      label: Contribution
      options:
        - label: I am willing to open a PR to fix this
          required: false
