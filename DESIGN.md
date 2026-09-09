# Design Document
Details design choices.  
This codebase should be written in idiomatic Rust.  

## For Loops
For loops should be used when the primary goal is to execute side effects for every 
element in a collection of stuff and the loop logic doesn't cleanly fit into a 
functional iterator adapter like `.map()`, `.filter()`, etc.
