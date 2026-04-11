## 1 https://openlibrary.org/trending/daily.json

{
    "$schema": "http://json-schema.org/draft-06/schema#",
    "$ref": "#/definitions/Welcome7",
    "definitions": {
        "Welcome7": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "query": {
                    "type": "string"
                },
                "works": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Work"
                    }
                },
                "days": {
                    "type": "integer"
                },
                "hours": {
                    "type": "integer"
                }
            },
            "required": [
                "days",
                "hours",
                "query",
                "works"
            ],
            "title": "Welcome7"
        },
        "Work": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "author_key": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "author_name": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "cover_edition_key": {
                    "type": "string"
                },
                "cover_i": {
                    "type": "integer"
                },
                "ebook_access": {
                    "$ref": "#/definitions/EbookAccess"
                },
                "edition_count": {
                    "type": "integer"
                },
                "first_publish_year": {
                    "type": "integer"
                },
                "has_fulltext": {
                    "type": "boolean"
                },
                "ia": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "ia_collection": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "key": {
                    "type": "string"
                },
                "language": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "lending_edition_s": {
                    "type": "string"
                },
                "lending_identifier_s": {
                    "type": "string"
                },
                "public_scan_b": {
                    "type": "boolean"
                },
                "title": {
                    "type": "string"
                },
                "editions": {
                    "$ref": "#/definitions/Editions"
                },
                "providers": {
                    "type": "array",
                    "items": {}
                },
                "series_key": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "series_name": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "series_position": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "subtitle": {
                    "type": "string"
                },
                "id_project_gutenberg": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "format": "integer"
                    }
                },
                "id_standard_ebooks": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "id_librivox": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "id_wikisource": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                }
            },
            "required": [
                "author_key",
                "author_name",
                "cover_i",
                "ebook_access",
                "edition_count",
                "editions",
                "first_publish_year",
                "has_fulltext",
                "key",
                "language",
                "providers",
                "public_scan_b",
                "title"
            ],
            "title": "Work"
        },
        "Editions": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "numFound": {
                    "type": "integer"
                },
                "start": {
                    "type": "integer"
                },
                "numFoundExact": {
                    "type": "boolean"
                },
                "docs": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Doc"
                    }
                }
            },
            "required": [
                "docs",
                "numFound",
                "numFoundExact",
                "start"
            ],
            "title": "Editions"
        },
        "Doc": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "key": {
                    "type": "string"
                },
                "title": {
                    "type": "string"
                },
                "subtitle": {
                    "type": "string"
                },
                "cover_i": {
                    "type": "integer"
                },
                "language": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "ia": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "ia_collection": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "ebook_access": {
                    "$ref": "#/definitions/EbookAccess"
                },
                "has_fulltext": {
                    "type": "boolean"
                },
                "public_scan_b": {
                    "type": "boolean"
                },
                "providers": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Provider"
                    }
                },
                "availability": {
                    "$ref": "#/definitions/Availability"
                },
                "id_standard_ebooks": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "id_librivox": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "format": "integer"
                    }
                }
            },
            "required": [
                "ebook_access",
                "has_fulltext",
                "key",
                "providers",
                "public_scan_b",
                "title"
            ],
            "title": "Doc"
        },
        "Availability": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "status": {
                    "$ref": "#/definitions/Status"
                },
                "available_to_browse": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "available_to_borrow": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "available_to_waitlist": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "is_printdisabled": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "is_readable": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "is_lendable": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "is_previewable": {
                    "type": "boolean"
                },
                "identifier": {
                    "type": "string"
                },
                "isbn": {
                    "anyOf": [
                        {
                            "type": "null"
                        },
                        {
                            "type": "string"
                        }
                    ]
                },
                "oclc": {
                    "type": "null"
                },
                "openlibrary_work": {
                    "anyOf": [
                        {
                            "type": "null"
                        },
                        {
                            "type": "string"
                        }
                    ]
                },
                "openlibrary_edition": {
                    "anyOf": [
                        {
                            "type": "null"
                        },
                        {
                            "type": "string"
                        }
                    ]
                },
                "last_loan_date": {
                    "anyOf": [
                        {
                            "type": "string",
                            "format": "date-time"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "num_waitlist": {
                    "anyOf": [
                        {
                            "type": "string",
                            "format": "integer"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "last_waitlist_date": {
                    "anyOf": [
                        {
                            "type": "string",
                            "format": "date-time"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "is_restricted": {
                    "type": "boolean"
                },
                "is_browseable": {
                    "anyOf": [
                        {
                            "type": "boolean"
                        },
                        {
                            "type": "null"
                        }
                    ]
                },
                "__src__": {
                    "$ref": "#/definitions/Src"
                }
            },
            "required": [
                "__src__",
                "available_to_borrow",
                "available_to_browse",
                "available_to_waitlist",
                "identifier",
                "is_browseable",
                "is_lendable",
                "is_previewable",
                "is_printdisabled",
                "is_readable",
                "is_restricted",
                "isbn",
                "last_loan_date",
                "last_waitlist_date",
                "num_waitlist",
                "oclc",
                "openlibrary_edition",
                "openlibrary_work",
                "status"
            ],
            "title": "Availability"
        },
        "Provider": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "access": {
                    "$ref": "#/definitions/Access"
                },
                "format": {
                    "$ref": "#/definitions/Format"
                },
                "price": {
                    "type": "null"
                },
                "url": {
                    "type": "string",
                    "format": "uri",
                    "qt-uri-protocols": [
                        "https"
                    ],
                    "qt-uri-extensions": [
                        ".epub",
                        ".pdf"
                    ]
                },
                "provider_name": {
                    "anyOf": [
                        {
                            "$ref": "#/definitions/ProviderName"
                        },
                        {
                            "type": "null"
                        }
                    ]
                }
            },
            "required": [
                "access",
                "format",
                "price",
                "provider_name",
                "url"
            ],
            "title": "Provider"
        },
        "EbookAccess": {
            "type": "string",
            "enum": [
                "borrowable",
                "printdisabled",
                "public",
                "no_ebook"
            ],
            "title": "EbookAccess"
        },
        "Src": {
            "type": "string",
            "enum": [
                "core.models.lending.get_availability"
            ],
            "title": "Src"
        },
        "Status": {
            "type": "string",
            "enum": [
                "borrow_available",
                "private",
                "borrow_unavailable",
                "open"
            ],
            "title": "Status"
        },
        "Access": {
            "type": "string",
            "enum": [
                "borrow",
                "sample",
                "open-access"
            ],
            "title": "Access"
        },
        "Format": {
            "type": "string",
            "enum": [
                "web",
                "pdf",
                "epub",
                "audio"
            ],
            "title": "Format"
        },
        "ProviderName": {
            "type": "string",
            "enum": [
                "ia",
                "standard_ebooks",
                "librivox"
            ],
            "title": "ProviderName"
        }
    }
}

## 2 https://openlibrary.org/trending/weekly.json

Estrutura json idêntica ao daily.json

## 3 https://openlibrary.org/subjects/{subject}.json?limit={n}&offset={n}

{
    "$schema": "http://json-schema.org/draft-06/schema#",
    "$ref": "#/definitions/Welcome1",
    "definitions": {
        "Welcome1": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "key": {
                    "type": "string"
                },
                "name": {
                    "type": "string"
                },
                "subject_type": {
                    "type": "string"
                },
                "solr_query": {
                    "type": "string"
                },
                "work_count": {
                    "type": "integer"
                },
                "works": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Work"
                    }
                }
            },
            "required": [
                "key",
                "name",
                "solr_query",
                "subject_type",
                "work_count",
                "works"
            ],
            "title": "Welcome1"
        },
        "Work": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "key": {
                    "type": "string"
                },
                "title": {
                    "type": "string"
                },
                "edition_count": {
                    "type": "integer"
                },
                "cover_id": {
                    "type": "integer"
                },
                "cover_edition_key": {
                    "type": "string"
                },
                "subject": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "ia_collection": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "printdisabled": {
                    "type": "boolean"
                },
                "lending_edition": {
                    "type": "string"
                },
                "lending_identifier": {
                    "type": "string"
                },
                "authors": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Author"
                    }
                },
                "first_publish_year": {
                    "type": "integer"
                },
                "ia": {
                    "type": "string"
                },
                "public_scan": {
                    "type": "boolean"
                },
                "has_fulltext": {
                    "type": "boolean"
                },
                "availability": {
                    "$ref": "#/definitions/Availability"
                }
            },
            "required": [
                "authors",
                "availability",
                "cover_edition_key",
                "cover_id",
                "edition_count",
                "first_publish_year",
                "has_fulltext",
                "ia",
                "ia_collection",
                "key",
                "lending_edition",
                "lending_identifier",
                "printdisabled",
                "public_scan",
                "subject",
                "title"
            ],
            "title": "Work"
        },
        "Author": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "key": {
                    "type": "string"
                },
                "name": {
                    "type": "string"
                }
            },
            "required": [
                "key",
                "name"
            ],
            "title": "Author"
        },
        "Availability": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "status": {
                    "type": "string"
                },
                "available_to_browse": {
                    "type": "boolean"
                },
                "available_to_borrow": {
                    "type": "boolean"
                },
                "available_to_waitlist": {
                    "type": "boolean"
                },
                "is_printdisabled": {
                    "type": "boolean"
                },
                "is_readable": {
                    "type": "boolean"
                },
                "is_lendable": {
                    "type": "boolean"
                },
                "is_previewable": {
                    "type": "boolean"
                },
                "identifier": {
                    "type": "string"
                },
                "isbn": {
                    "type": "null"
                },
                "oclc": {
                    "type": "null"
                },
                "openlibrary_work": {
                    "type": "string"
                },
                "openlibrary_edition": {
                    "type": "string"
                },
                "last_loan_date": {
                    "type": "null"
                },
                "num_waitlist": {
                    "type": "null"
                },
                "last_waitlist_date": {
                    "type": "null"
                },
                "is_restricted": {
                    "type": "boolean"
                },
                "is_browseable": {
                    "type": "boolean"
                },
                "__src__": {
                    "type": "string"
                }
            },
            "required": [
                "__src__",
                "available_to_borrow",
                "available_to_browse",
                "available_to_waitlist",
                "identifier",
                "is_browseable",
                "is_lendable",
                "is_previewable",
                "is_printdisabled",
                "is_readable",
                "is_restricted",
                "isbn",
                "last_loan_date",
                "last_waitlist_date",
                "num_waitlist",
                "oclc",
                "openlibrary_edition",
                "openlibrary_work",
                "status"
            ],
            "title": "Availability"
        }
    }
}

## 4 https://openlibrary.org/works/{work_id}.json

{
    "$schema": "http://json-schema.org/draft-06/schema#",
    "$ref": "#/definitions/Welcome9",
    "definitions": {
        "Welcome9": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "description": {
                    "type": "string"
                },
                "title": {
                    "type": "string"
                },
                "covers": {
                    "type": "array",
                    "items": {
                        "type": "integer"
                    }
                },
                "first_publish_date": {
                    "type": "string"
                },
                "key": {
                    "type": "string"
                },
                "authors": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Author"
                    }
                },
                "type": {
                    "$ref": "#/definitions/Type"
                },
                "subjects": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "dewey_number": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "links": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Link"
                    }
                },
                "latest_revision": {
                    "type": "integer"
                },
                "revision": {
                    "type": "integer"
                },
                "created": {
                    "$ref": "#/definitions/Created"
                },
                "last_modified": {
                    "$ref": "#/definitions/Created"
                }
            },
            "required": [
                "authors",
                "covers",
                "created",
                "description",
                "dewey_number",
                "first_publish_date",
                "key",
                "last_modified",
                "latest_revision",
                "links",
                "revision",
                "subjects",
                "title",
                "type"
            ],
            "title": "Welcome9"
        },
        "Author": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "author": {
                    "$ref": "#/definitions/Type"
                },
                "type": {
                    "$ref": "#/definitions/Type"
                }
            },
            "required": [
                "author",
                "type"
            ],
            "title": "Author"
        },
        "Type": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "key": {
                    "type": "string"
                }
            },
            "required": [
                "key"
            ],
            "title": "Type"
        },
        "Created": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "type": {
                    "type": "string"
                },
                "value": {
                    "type": "string",
                    "format": "date-time"
                }
            },
            "required": [
                "type",
                "value"
            ],
            "title": "Created"
        },
        "Link": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "title": {
                    "type": "string"
                },
                "url": {
                    "type": "string",
                    "format": "uri",
                    "qt-uri-protocols": [
                        "https"
                    ]
                },
                "type": {
                    "$ref": "#/definitions/Type"
                }
            },
            "required": [
                "title",
                "type",
                "url"
            ],
            "title": "Link"
        }
    }
}

## 5 https://openlibrary.org/works/{work_id}/editions.json?limit={n}&offset={n}

{
    "$schema": "http://json-schema.org/draft-06/schema#",
    "$ref": "#/definitions/Welcome7",
    "definitions": {
        "Welcome7": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "links": {
                    "$ref": "#/definitions/Links"
                },
                "size": {
                    "type": "integer"
                },
                "entries": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Entry"
                    }
                }
            },
            "required": [
                "entries",
                "links",
                "size"
            ],
            "title": "Welcome7"
        },
        "Entry": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "works": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/TypeElement"
                    }
                },
                "title": {
                    "type": "string"
                },
                "publishers": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "publish_date": {
                    "type": "string"
                },
                "key": {
                    "type": "string"
                },
                "type": {
                    "$ref": "#/definitions/TypeElement"
                },
                "identifiers": {
                    "$ref": "#/definitions/Identifiers"
                },
                "isbn_10": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "latest_revision": {
                    "type": "integer"
                },
                "revision": {
                    "type": "integer"
                },
                "created": {
                    "$ref": "#/definitions/Created"
                },
                "last_modified": {
                    "$ref": "#/definitions/Created"
                },
                "authors": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/TypeElement"
                    }
                },
                "isbn_13": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "languages": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/TypeElement"
                    }
                },
                "number_of_pages": {
                    "type": "integer"
                },
                "source_records": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                },
                "weight": {
                    "type": "string"
                },
                "subtitle": {
                    "type": "string"
                },
                "full_title": {
                    "type": "string"
                },
                "covers": {
                    "type": "array",
                    "items": {
                        "type": "integer"
                    }
                },
                "pagination": {
                    "type": "string",
                    "format": "integer"
                }
            },
            "required": [
                "created",
                "key",
                "last_modified",
                "latest_revision",
                "publish_date",
                "publishers",
                "revision",
                "title",
                "type",
                "works"
            ],
            "title": "Entry"
        },
        "TypeElement": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "key": {
                    "type": "string"
                }
            },
            "required": [
                "key"
            ],
            "title": "TypeElement"
        },
        "Created": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "type": {
                    "$ref": "#/definitions/TypeEnum"
                },
                "value": {
                    "type": "string",
                    "format": "date-time"
                }
            },
            "required": [
                "type",
                "value"
            ],
            "title": "Created"
        },
        "Identifiers": {
            "type": "object",
            "additionalProperties": false,
            "title": "Identifiers"
        },
        "Links": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "self": {
                    "type": "string"
                },
                "work": {
                    "type": "string"
                },
                "next": {
                    "type": "string"
                }
            },
            "required": [
                "next",
                "self",
                "work"
            ],
            "title": "Links"
        },
        "TypeEnum": {
            "type": "string",
            "enum": [
                "/type/datetime"
            ],
            "title": "TypeEnum"
        }
    }
}