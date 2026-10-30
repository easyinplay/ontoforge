export interface paths {
    "/api/actions/{action}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["action"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/aggregate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["aggregate"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/commands/{command_id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["command_result"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/objects/{object_type}/{pk}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["object"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/objects/{object_type}/{pk}/history": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["history"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/ontology": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["ontology"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/query": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["query"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/health": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["health"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}

export type webhooks = Record<string, never>;

export interface components {
    schemas: {
        ActionItem: {
            expectedVersions?: {
                [key: string]: string;
            };
            params: {
                [key: string]: unknown;
            };
        };
        ActionMetadata: {
            apiName: string;
            description?: string | null;
            displayName?: string | null;
        };
        ActionReceipt: {
            chosenCurrentTime: string;
            commandId: string;
            committedAt: string;
            eventId: string;
            items: components["schemas"]["ItemReceipt"][];
            /** Format: int64 */
            logSeq: bigint;
        };
        ActionRequest: {
            commandId?: string | null;
            items: components["schemas"]["ActionItem"][];
            validateOnly?: boolean;
        };
        AggregateBucket: {
            group: unknown[];
            values: unknown[];
        };
        AggregateMeasure: {
            /** @enum {string} */
            op: "count";
            property?: string | null;
        } | {
            /** @enum {string} */
            op: "sum";
            property: string;
        } | {
            /** @enum {string} */
            op: "avg";
            property: string;
        } | {
            /** @enum {string} */
            op: "min";
            property: string;
        } | {
            /** @enum {string} */
            op: "max";
            property: string;
        } | {
            /** @enum {string} */
            op: "distinct";
            property: string;
        };
        AggregateRequest: {
            groupBy?: string[];
            measures: components["schemas"]["AggregateMeasure"][];
            set: components["schemas"]["ObjectSet"];
        };
        AggregateResult: {
            buckets: components["schemas"]["AggregateBucket"][];
        };
        /** @enum {string} */
        Direction: "asc" | "desc";
        /** @enum {string} */
        EngineAvailability: "available" | "unavailable";
        ErrorEnvelope: {
            apiName: string;
            code: string;
            request_id: string;
        };
        HealthResponse: {
            engineStatus: components["schemas"]["EngineAvailability"];
            status: string;
        };
        HistoryEntry: {
            action: string;
            actor: string;
            committedAt: string;
            eventId: string;
            maskedChanges: string[];
            op: string;
            params?: unknown;
            /** Format: int64 */
            seq: bigint;
            unset: string[];
            values: {
                [key: string]: unknown;
            };
            /** Format: int64 */
            version: bigint;
        };
        HistoryPage: {
            entries: components["schemas"]["HistoryEntry"][];
            /** Format: int64 */
            nextBeforeSeq?: bigint | null;
            truncated: boolean;
        };
        ItemReceipt: {
            /** Format: int32 */
            itemOrdinal: bigint;
            objectType: string;
            pk: unknown;
            version: string;
        };
        /** @enum {string} */
        LinkDirection: "fromTo" | "toFrom";
        LinkMetadata: {
            apiName: string;
            description?: string | null;
            displayName?: string | null;
            from: string;
            to: string;
        };
        ListPage: {
            cursor?: string | null;
            hasMore: boolean;
            rows: components["schemas"]["ObjectRow"][];
            truncated: boolean;
        };
        Metadata: {
            actions: components["schemas"]["ActionMetadata"][];
            apiName: string;
            /** Format: int64 */
            defVersion: bigint;
            description?: string | null;
            displayName?: string | null;
            engineStatus: components["schemas"]["EngineAvailability"];
            links: components["schemas"]["LinkMetadata"][];
            objectTypes: components["schemas"]["ObjectMetadata"][];
        };
        /** @enum {string} */
        Nulls: "first" | "last";
        ObjectMetadata: {
            aliases: string[];
            apiName: string;
            description?: string | null;
            displayName?: string | null;
            primaryKey: string;
            properties: components["schemas"]["PropertyMetadata"][];
            titleKey: string;
        };
        ObjectRow: {
            _version: string;
            objectType: string;
            pk: unknown;
            properties: {
                [key: string]: unknown;
            };
        };
        ObjectSet: {
            /** @enum {string} */
            kind: "pk";
            object_type: string;
            pk: unknown;
        } | {
            /** @enum {string} */
            kind: "type";
            object_type: string;
        } | {
            /** @enum {string} */
            kind: "where";
            object_type: string;
            predicate: components["schemas"]["WhereClause"];
        } | {
            /** @enum {string} */
            kind: "search";
            mode: components["schemas"]["TermMode"];
            object_type: string;
            property: string;
            query: string;
        } | {
            /** @enum {string} */
            kind: "union";
            object_type: string;
            sets: components["schemas"]["ObjectSet"][];
        } | {
            /** @enum {string} */
            kind: "intersect";
            object_type: string;
            sets: components["schemas"]["ObjectSet"][];
        } | {
            /** @enum {string} */
            kind: "subtract";
            left: components["schemas"]["ObjectSet"];
            object_type: string;
            right: components["schemas"]["ObjectSet"];
        } | {
            base: components["schemas"]["ObjectSet"];
            direction: components["schemas"]["LinkDirection"];
            /** @enum {string} */
            kind: "searchAround";
            link: string;
            object_type: string;
        };
        OrderBy: {
            direction: components["schemas"]["Direction"];
            nulls: components["schemas"]["Nulls"];
            property: string;
        };
        PageSpec: {
            after?: string | null;
            /** Format: int64 */
            first: bigint;
        };
        PropertyMetadata: {
            apiName: string;
            description?: string | null;
            displayName?: string | null;
            required: boolean;
            type: string;
        };
        QueryRequest: {
            orderBy?: components["schemas"]["OrderBy"][];
            page: components["schemas"]["PageSpec"];
            select?: string[];
            set: components["schemas"]["ObjectSet"];
        };
        /** @enum {string} */
        TermMode: "any" | "all";
        ValidationResponse: {
            validated: boolean;
        };
        WhereClause: {
            /** @enum {string} */
            op: "eq";
            property: string;
            value: unknown;
        } | {
            /** @enum {string} */
            op: "gt";
            property: string;
            value: unknown;
        } | {
            /** @enum {string} */
            op: "gte";
            property: string;
            value: unknown;
        } | {
            /** @enum {string} */
            op: "lt";
            property: string;
            value: unknown;
        } | {
            /** @enum {string} */
            op: "lte";
            property: string;
            value: unknown;
        } | {
            /** @enum {string} */
            op: "isNull";
            property: string;
        } | {
            /** @enum {string} */
            op: "in";
            property: string;
            values: unknown[];
        } | {
            /** @enum {string} */
            op: "startsWith";
            property: string;
            value: string;
        } | {
            /** @enum {string} */
            op: "containsAnyTerm";
            property: string;
            value: string;
        } | {
            /** @enum {string} */
            op: "containsAllTerms";
            property: string;
            value: string;
        } | {
            clauses: components["schemas"]["WhereClause"][];
            /** @enum {string} */
            op: "and";
        } | {
            clauses: components["schemas"]["WhereClause"][];
            /** @enum {string} */
            op: "or";
        } | {
            clause: components["schemas"]["WhereClause"];
            /** @enum {string} */
            op: "not";
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}

export type $defs = Record<string, never>;

export interface operations {
    action: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Action API name */
                action: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ActionRequest"];
            };
        };
        responses: {
            /** @description Committed receipt or validation acknowledgement */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": unknown;
                };
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    aggregate: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["AggregateRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["AggregateResult"];
                };
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    command_result: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Durable Action command id */
                command_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ActionReceipt"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    object: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Logical object type API name */
                object_type: string;
                /** @description JSON or string primary key */
                pk: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ObjectRow"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    history: {
        parameters: {
            query?: {
                /** @description Maximum history entries */
                limit?: bigint;
                /** @description Return entries before sequence */
                beforeSeq?: bigint;
            };
            header?: never;
            path: {
                /** @description Logical object type API name */
                object_type: string;
                /** @description JSON or string primary key */
                pk: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HistoryPage"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    ontology: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Scoped ontology metadata */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Metadata"];
                };
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    query: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["QueryRequest"];
            };
        };
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ListPage"];
                };
            };
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ErrorEnvelope"];
                };
            };
        };
    };
    health: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HealthResponse"];
                };
            };
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["HealthResponse"];
                };
            };
        };
    };
}
