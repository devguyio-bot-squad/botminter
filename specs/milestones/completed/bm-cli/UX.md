```plaintext
$ bm init
bm> Where do you want your workspaces?[~/.botminter/workspaces]
~/.botminter/workspaces will be created
bm> Team name?: hypershift
...
bm> Profile?
  1. rh-scrum
  2. compact
Enter your choice [1]: 
...
bm> GitHub team repo?: github.com/bot-squad/team-repo
Checking if team repo existst
bm> Your team repo does not exist. Want me to create and bootstrap it during initialization (y/n)?[y]: 
...
bm> Want to hire members? (available roles: human-assistant, architect)
...
bm> Want to add projects?
...
I have all the information I need to start initialization. 

Summary:
....

Proceed (y/n)?: y
...
Created bla bla

$ bm teams list
- hypershift (default)
- cluster-api-team

$ bm start -t hypershift # team is optional, defaults to default team (alias: bm up)
Bringing your team to life

$ bm status
Team hypershift
===============

Profile: rh-scrum
  
Members
-------

1) Human-Assistant:

Status: Alive
<some ralph status>

2) Architect:
   
...

$ bm members list -t hypershift # team is optional, to default team
- architect
- dev
- human assistant

$ bm knowledge # since no verb was entered, we go into interactive mode where we can do CRUD operations
Team Knowledge
```

## profile schema

- 
- profiles/
    - rh-scrum/
        - botminter.yml # similar to Chart.yaml , contains description, display name, schema version (we only have 1 for now), version
        - .schema/
            - v1.yml
                - ```                  
                  name: v1
                  team:
                      teamKnowledge: "knowledge" #location of knowledge 
                      teamInvariants: "invariants" #location of invariants
                      projects: "projects" #location of projects
                  member:
                      teamKnowledge: "knowledge" #location of knowledge 
                      teamInvariants: "invariants" #location of invariants
                      projects: "projects" #location of projects
                  ```
        - agent/
        - invariants/
        - knowledge/
        - members/
            - architect/
            - ....
        - CLAUDE.md
        - PROCESS.md