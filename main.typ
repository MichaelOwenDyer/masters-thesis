#import "content/metadata.typ": metadata
#import "thesis.typ": thesis

#set document(
  title: metadata.title,
  author: metadata.author,
)
#thesis()
