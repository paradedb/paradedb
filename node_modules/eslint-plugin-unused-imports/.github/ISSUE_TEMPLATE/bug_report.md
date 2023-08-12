---
name: Bug report
about: Create a report to help us improve
title: ''
labels: ''
assignees: ''

---

# Please follow the general troubleshooting steps first:

If the issue is with something being marked wrongly as a unused import and therefore being removed. It is an issue with the imported package (`@typescript-eslint/eslint-plugin` for TS or `eslint` for JS) and its `no-unused-vars` rule. I cannot do anything about this except updating if a fix is made upstream. 

If new rules are added `no-unused-vars` upstream which should be autofixed, mark your issue *rule addition*.

Now if something is not marked an import and being removed by the autofixer, it is an issue I can do something about.

Please replace the above with a brief summary of your issue.

### Features:

**Please note by far the quickest way to get a new feature is to file a Pull Request.**
