# Selecting commits for `cargo vet` audits

When auditing the changes between two versions of a crate for `cargo
vet`, in practice it is usually not necessary to examine every commit:
if a Mozillian or other trusted contributor is the author of a commit,
or reviewed the change, then we can consider that commit covered by
the organization's review standards.

However, determining who reviewed and merged commits for projects
developed on GitHub can take a lot of mouse clicks. This repo contains
scripts for producing a table of commits, sorted by pull request, with
author, reviewer, and committer names, suitable for uploading to
Google Sheets, where it can be used by a team to coordinate vetting
work.

## Workflow overview

There are generally three phases to producing a vetting table:

-   First, produce a list of the commits that need to be audited. This
    is typically just a git command:
  
        $ git rev-list gfx-rs/v0.12..gfx-rs/v0.13 > commit-list
        
-   Second, gather information from GitHub about those commits using the
    GitHub REST API. Specifically, we want data from GitHub's `commits`,
    `pulls`, and `reviews` endpoints.

-   Finally, combine the data above to produce a table of the commits
    together with the information needed to decide which of them require
    an audit.

Each of these phases is described in more detail below.

## Prerequisites

In these instructions, we'll assume you have this repository checked
out in some directory named `$scripts`.

The scripts require that you have the following tools installed:

-   The GitHub command-line interface, `gh`. It might be possible to
    do things directly with `curl` or `wget`, but `gh` handles
    authentication and maybe some other things, so I didn't try too
    hard to see if we could avoid it.
    
    Make sure you've authenticated yourself:
    
        $ gh auth status
        github.com
          ✓ Logged in to github.com as jimblandy (oauth_token)
          ✓ Git operations for github.com configured to use ssh protocol.
          ✓ Token: *******************
  
    The `gh` tool is packaged by Fedora.

-   The `jq` JSON processing tool. This is what we use to combine the
    data we get from the GitHub REST API into the form we need. If you
    don't know `jq`, definitely check it out the next time you have
    some JSON you need to deal with. (See also the `jaq` Rust crate.)

Here is a command to install the prerequisites on Fedora:

    $ sudo dnf install gh jq

### Work directory layout

Start with an empty directory, which will hold the data fetched from
GitHub and various other intermediate results:

    $ mkdir vet-myproj

We'll call the full path to this directory `$work`. These scripts
assume that `$work` is the current directory when they are invoked.

    $ cd vet-myproj

When the whole process is complete, we will have a tree like this:

    vet-myproj
    ├── repo.sh
    ├── trusted.json
    ├── commit-list
    ├── commit-pulls.json
    ├── mergers-and-approvers.tsv
    ├── pull-list.json
    ├── pulls.json
    ├── reviews.json
    ├── commits
    │   ├── 006bbbc94d49b2920188cbdadf97802d064494be
    │   ├── 01628a1fad05708ebc3d5e701736915b39ad37ae
    │   ..
    │   ├── fd954a2bd6e19e2954495e03ca171d4a1264a2d5
    │   └── ff07716f79fb1d84a895bca9d0534947f024294f
    ├── commit-pulls
    │   ├── 006bbbc94d49b2920188cbdadf97802d064494be
    │   ├── 01628a1fad05708ebc3d5e701736915b39ad37ae
    │   ..
    │   ├── fd954a2bd6e19e2954495e03ca171d4a1264a2d5
    │   └── ff07716f79fb1d84a895bca9d0534947f024294f
    ├── pulls
    │   ├── 2303
    │   ├── 2305
    │   ...
    │   ├── 2816
    │   └── 2817
    └── reviews
        ├── 2303
        ├── 2305
        ...
        ├── 2816
        └── 2817

In this tree:

- The `repo.sh` file indicates which GitHub repository we're working
  with.

- The `trusted.json` file indicates which contributors we're treating
  as trusted authors and reviewers.

- The `commit-list` file is a list of git long hashes of the commits
  we need to review. These are up to you; it could simply be the
  output from `git rev-list OLD..NEW`. But it should contain long
  commit SHAs, not short hashes.

- The `commits`, `pulls` and `reviews` subdirectories hold the raw
  results of individual queries from the GitHub RUST API, produced by
  the `github-fetch.sh` script, based on `commit-list`. Hopefully, you
  can retrieve these once and then leave them alone, to keep GitHub
  from rate-limiting you.

- The `.json` files hold combined intermediate results used by the
  `mergers-and-approvers.sh` script.

- The `mergers-and-approvers.tsv` file is the final result, ready for
  importing into Google Sheets. This is produced by
  `mergers-and-approvers.sh`.

## Creating the config file

The scripts that retrieve data from GitHub need to know which
repository to operate on. For this, they consult the file
`$work/repo.sh`, whose contents look like this:

    owner=gfx-rs
    repo=naga
    
These specify the GitHub owner and their repository the scripts should
access. This file is `source`-ed by each script when it starts.

## Creating the trusted reviewers table

The point of this exercise is to identify commits that we don't need
to audit because they were either authored or reviewed by people we
trust to apply the appropriate standards of review. The `trusted.json`
file identifies who these trusted people are.

The file should contain a JSON object whose keys are the GitHub
usernames of trusted authors/reviewers. The values of the keys don't
matter. For example:

    {
      "jimblandy": true,
      "kvark": true,
      "nical": true
    }

## Generating the commit list

Although `cargo vet` operates in terms of crate versions, the auditing
process described here works in terms of git commits - and there is no
straightforward relationship between crate versions and git commits.
If the crate's maintainers happen to have placed a tag on the commit
from which they published the crate, that's great, but there's no
automation that ensures that that actually happens reliably. So it's
up to you to identify an appropriate range of commits, and check that
it corresponds to what was actually published.

In this example, assume that we want to audit commits reachable from
`gfx-rs/v0.13` that are not reachable from `gfx-rs/v0.12`, which has
already been audited. If `$source` is a git checkout of the project
we're auditing, we would generate the `commit-list` file with a
command like:

    $ git -C $source gfx-rs/v0.12..gfx-rs/v0.13 > commit-list

The `-C` option just tells git to behave as if it were started in the
given directory.

## Fetching commit data from GitHub

To fetch the list of pulls and reviews associated with each commit:

    $ sh $scripts/fetch-commits.sh

This is the first step that contacts GitHub, and all it does is file
away the data retrieved with minimal processing (just pretty-printing
the JSON). All the steps that hit the network are separated out into
their own scripts, so that the steps that do interesting processing
can be iterated on without constantly hitting the network.

The `fetch-commits.sh` script sleeps a bit between requests to avoid
getting rate-limited. I have never actually had a problem with this,
but sometimes we have hundreds of commits to retrieve, and it just
seems polite.

## Finding pull requests for each commit

To find the set of pull requests (usually only one) associated with
each commit, run the `$scripts/make-commit-pulls.sh` script:

    $ sh $scripts/make-commit-pulls.sh

This creates the file `$work/commit-pulls.json`, which is a JSON
object mapping commit SHAs to lists of pull request numbers, like
this:

    {
      "06ae90527dcede1a98d6f15fa7c440f0eab5d0ba": {
        "pulls": [
          1998
        ]
      },
      "27d38aae33fdbfa72197847038cb470720594cb1": {
        "pulls": [
          1989
        ]
      },
      "67ef37ae991f72f06a58774c3866d716d1c9a9c1": {
        "pulls": [
          1993
        ]
      },
      "7555df952e45969d82ac260caa49a1b7beacfe7e": {
        "pulls": []
      },
      "9b7fe8803db1c8bb21ee47bd6691f0cd72ef28cc": {
        "pulls": []
      },
      "b746e0a4209133c0654d0c8959db97b45cf9358a": {
        "pulls": [
          1995
        ]
      },
      "e2d688088a8e900e22da348cdc7ba0655394b498": {
        "pulls": [
          1933
        ]
      }
    }

There may be some commits that have no associated pull requests. If
so, the script lists them. The next section explains how to deal with
this.

### Commits without pull requests

Some commits are not associated with any pull request - say, if
someone simply pushes a commit directly to the repository. This
results in a file in the `commit-pulls` directory containing only an
empty JSON array:

    []
    
This naturally leads to an empty list of pull requests for that commit
SHA in `$work/commit-pulls.json` as well.

However, the GitHub REST API also has a bug which makes it unable to
find the pull request associated with a given commit in some cases.
GitHub seems to return a response like the above when a PR is squashed
and rebased via the web interface before being landed. See
[#1](https://github.com/jimblandy/vet-wgpu/issues/1) for a bit more
detail.

In the sample `$work/commit-pulls.json` file shown above, you can see
that while five of the commits have an associated pull request,
`7555df9` and `9b7fe88` do not. To see which commits have no pull
requests in your own data, use a query like this:

    $ jq 'map_values(select(.pulls == []))' commit-pulls.json
    {
      "7555df952e45969d82ac260caa49a1b7beacfe7e": {
        "pulls": []
      },
      "9b7fe8803db1c8bb21ee47bd6691f0cd72ef28cc": {
        "pulls": []
      }
    }
    $

After investigation, suppose you determine that `7555df9` indeed has no
associated pull request, but `9b7fe88` ought to be associated with PR
1862. You can create a file `$work/commit-pulls-overrides.json`, of the
same format as `commit-pulls.json`, that specifies only the commits
for which you want to override the pull list:

    {
      "9b7fe8803db1c8bb21ee47bd6691f0cd72ef28cc": {
        "pulls": [1862]
      }
    }

The keys in this object must exactly match the keys in the
`commit-pulls.json` object; short hashes are not acceptable in
`commit-pulls-overrides.json`, because `commit-pulls.json` doesn't use
short hashes.

## Fetching pull request and review data from GitHub

Once you're satisfied with the pull request lists in
`$work/commit-pulls.json` and any adjustments in
`$work/commit-pulls-overrides.json`, you can run the `fetch-pulls.sh`
script to retrieve data about the pull requests and their reviews from
GitHub:

    $ sh $scripts/fetch-pulls.sh
    
## Generating the mergers and approvers table

Once the data has been retrieved, run the `mergers-and-approvers.sh`
script to write the `mergers-and-approvers.tsv` file, which can be
imported into Google Sheets or something like that.

    $ sh $scripts/mergers-and-approvers
    $ cat mergers-and-approvers.tsv 
    1933	e2d688088a8e900e22da348cdc7ba0655394b498	expenses	JCapucho	JCapucho	
    1989	27d38aae33fdbfa72197847038cb470720594cb1	teoxoy	jimblandy	jimblandy	jimblandy
    1993	67ef37ae991f72f06a58774c3866d716d1c9a9c1	JCapucho	jimblandy	jimblandy	jimblandy
    1995	b746e0a4209133c0654d0c8959db97b45cf9358a	JCapucho	cwfitzgerald	cwfitzgerald	
    1998	06ae90527dcede1a98d6f15fa7c440f0eab5d0ba	cwfitzgerald		cwfitzgerald	
    $
    
The columns here are:

- pull request number
- commit SHA
- author
- reviewers who approved the pull request
- person who merged the pull request

You can import this file into Google Sheets using *File > Import*, and
share it with your team to coordinate vetting work.

There is a Google Sheets [template][t] you can copy, if that helps.
You'll probably need to adjust some of the ranges to match the number
of rows in your data.

[t]: https://docs.google.com/spreadsheets/d/1R7gZonQf2lIXMkhnr-BuX8pvzEJtFqf1449icgKpX4w

## Deleting generated files

The script `cleanup.sh` deletes all files generated by these scripts.
It leaves alone the files you created, like `commit-list` and
`trusted.json`.
