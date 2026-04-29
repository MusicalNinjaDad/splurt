# Contributing

Feel free to raise an issue with any ideas. If you want to add something - the usual PR process applies. Please try to respect the feedback from the CI (linting, formatting, testing etc.). Make sure you have good meaningful tests in place. Your code will be read by people, not just machines - it's a lot easier to write code a machine understands than code that another human can understand - most of [PEP20](https://peps.python.org/pep-0020/) applies to all languages!

Don't worry too much about your personal commit style - I will squash the PR, so do what works best for you while you are coding. Do try to be nice to me as a reviewer though with a decent PR title and clear PR description.

If that sounds too much - raise a PR anyway, or a draft, @-tag me on it and start a discussion. I'm happy to help and co-author the final version.

Don't worry about changing existing code - anecdotal evidence shows that any code more than two weeks' old was always written by "not me" whatever git-blame says ;)

## Contributor Accessibility

If anything in the codebase could be changed to help with any accessibility needs you have - please let me know! (e.g. I need a CI to run on my fork - otherwise I get in a tangle forgetting to format, lint, test etc. every commit by hand.)

It's a small passion of mine to think about coder accessibility - so I'm interested anyway. I'd love to hear from any dyslexic coders about code layout, naming, etc ...

## Getting up to speed

1. Use the devcontainer ([what's that?](https://containers.dev/)). If it's missing something, please raise a dedicated PR.

If you really want a step-by-step instruction on what to install locally to work on the codebase - take a look at [the devcontainer DockerFile](.devcontainer/DockerFile) - it documents everything much better than I can here.

## Everything in git is first-class code

The header says it all. Documentation, examples, tests, implementation logic, even this md ... if it's in git I consider it `code` of equal value.

## Lints

These are there as reminders of things to think about, not as road-blocks. If there is automation in place that dislikes your code then don't hack around it.

If a lint complains add an `#[expect(lint, reason = "why")]`. Use `expect`, not allow so that the code doesn't end up with allow-zombies as things change. Add a decent reason.

Also see the point on AI-PR-Reviews below.

## AI Policy

### Generative AI: Own your code

Please **don't submit something where an LLM created the code** (see above for a definition of code). If you have a great idea but need to prompt something to create it - open an issue, post to [users.rust-lang.org](https://users.rust-lang.org) and prompt a human instead, that way you can help someone who want's to code *and* get your idea implemented.

(For clarity, if you dictate the code to an AI-agent, because you have a problem with typing ... *that's fine* - it's your code)

### AI assistance & accessibility: This is fine

It's fine if you use an AI:

- as a "sparring partner" to help you work out ideas and challenge your thinking
- as a "researcher" to help you find information and make it accessible to you
- to translate from your native language to English (but not to take a short prompt and make a lot of output)
- to check your work and give you feedback
- to make the codebase more accessible in any way

I do most of these too.

### AI PR Reviews (sourcery)

I have sourcery.ai set to review all PRs on all my repos. Why?

- Because I am ND and make a bucket-load of tiny mistakes: spelling errors, inconsistencies, etc. I find the hard things easy and the easy things hard.
- Because I believe it's always worth getting my own code reviewed and no-one is going to do that for me.
- Because I want a first-triage on anything coming in to the repo. I will read the comments from the bot & your responses to them when reviewing the PR.

The bot will provide you with feedback, think about it. You are the human. Usually it is worth adding some extra documentation, a todo & issue or at least a comment. Sometimes it's really valuable. Sometimes it's completely wrong.

It might tell you your code won't compile even when the CI passed, then catch 3 typos and an awkward API(!)
