# Contributing to the Astria monorepo

Before an author submits a patch to the Astria monorepo at https://github.com/astriaorg/astria,
they should make sure to take into account the guidelines laid out in this document.

## How Pull Requests are written at Astria

The following points represent an ideal scenario that authors of patches should try to
follow when submitting them in the form of GitHub pull requests (PRs):

0. Be kind and respectful or your readers. Code is written once and read many times. Be mindful
   that a good review can take as much or more time than writing code.
1. The pull request title follows [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/).
2. A patch should be written in such a way that it is easy for a reader to understand and review.
3. Prefer shorter over longer patches.
4. A patch should accomplish one goal, and accomplish it well.
5. To point 3, separate refactoring and feature development into separate PRs. If a feature
   requires the codebase be refactored, refactor first, implement the feature in a follow-up PR.
   The same applies to bugfixes.
6. When moving around code in refactors (for example, moving a test module into a separate file),
   do not also change item names or add new items. Remember that GitHub diffs do not help a reviewer
   track which code is new, deleted, or just moved.
7. PR title and summary should give a concise and accurate representation of what the patch
   wants to accomplish. They should not make assumptions about a reader knowing implementation details.
   Ideally, put these in doc comments that live next to the code.
8. If a code change does not help accomplish the patche's goal described in 7, it should be moved to
   another patch.
9. Code should not be surprising - it should be idiomatic and fit the rest of the code surrounding it.
   If a code changes is still necessary, flag it for the reader using an inline comment, or adding a
   github review comment pulling attention to it.
