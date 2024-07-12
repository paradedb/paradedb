name: Bug report
about: Create a report to help us improve
labels: ""
assignees: ""
title: ""
body:
  - type: textarea
    attributes:
      label: What happens?
      description: A short, clear and concise description of the bug.
      placeholder: Describe the bug
    validations:
      required: true

  - type: textarea
    attributes:
      label: To Reproduce
      description: |
        Please provide steps to reproduce the behaviour, preferably a [minimal reproducible example](https://en.wikipedia.org/wiki/Minimal_reproducible_example).
      placeholder: Steps to reproduce the behavior
    validations:
      required: true

  - type: markdown
    attributes:
      value: "### Environment (please complete the following information):"

  - type: input
    attributes:
      label: "OS:"
      placeholder: e.g., iOS
      description: Please include your operating system version and architecture (e.g., aarch64, x86, x64, etc)
    validations:
      required: true

  - type: input
    attributes:
      label: "ParadeDB Version:"
      placeholder: e.g., v0.8.0
    validations:
      required: true

  - type: markdown
    attributes:
      value: "### Identity Disclosure:"

  - type: input
    attributes:
      label: "Full Name:"
      placeholder: e.g., John Doe
    validations:
      required: true

  - type: input
    attributes:
      label: "Affiliation:"
      placeholder: e.g., Acme Corporation
    validations:
      required: true

  - type: markdown
    attributes:
      value: |
        If the above is not given and is not obvious from your GitHub profile page, we might close your issue without further review. Please refer to the [reasoning behind this rule](https://berthub.eu/articles/posts/anonymous-help/) if you have questions.

  - type: dropdown
    attributes:
      label: What is the latest build you tested with? If possible, we recommend testing by compiling the latest `dev` branch.
      description: |
        Visit the [README page](https://github.com/paradedb/paradedb) for instructions.
      options:
        - I have not tested with any build
        - I have tested with a stable release
        - I have tested with a source build
    validations:
      required: true

  - type: dropdown
    attributes:
      label: Did you include all relevant data sets for reproducing the issue?
      options:
        - No - Other reason (please specify in the issue body)
        - No - I cannot share the data sets because they are confidential
        - No - I cannot easily share my data sets due to their large size
        - Not applicable - the reproduction does not require a data set
        - Yes
      default: 0
    validations:
      required: true

  - type: checkboxes
    attributes:
      label: Did you include the code required to reproduce the issue?
      options:
        - label: Yes, I have

  - type: checkboxes
    attributes:
      label: Did you include all relevant configurations (e.g., CPU architecture, PostgreSQL version, Linux distribution) to reproduce the issue?
      options:
        - label: Yes, I have
