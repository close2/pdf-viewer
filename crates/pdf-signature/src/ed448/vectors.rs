//! RFC 8032 section 7.4's and section 7.5's vectors, transcribed by script from
//! `doc/md/rfc/rfc8032.txt` and checked by [`super::tests`]: every one verifies, and every one
//! stops verifying when one bit moves.
//!
//! The secret keys are not carried: a verifier never sees one, and these tests verify.
//!
//! **Taken from IETF RFC 8032** (Josefsson and Liusvaara, 2017; the IETF Trust's Legal Provisions
//! section 3.c.iii, unmodified portions attributed to the RFC they come from). The hexadecimal is
//! the RFC's own, joined across the lines it was printed on. `doc/third-party-data.md` records it.

/// One vector: the section's own heading, and the three values a verifier is handed.
pub(super) struct Vector {
    /// The heading the section prints after its dashes, e.g. `1 octet (with context)`.
    pub(super) name: &'static str,
    /// `PUBLIC KEY`, hexadecimal.
    pub(super) public_key: &'static str,
    /// `MESSAGE`, hexadecimal; empty for the `Blank` vector.
    pub(super) message: &'static str,
    /// `CONTEXT`, hexadecimal; empty where the vector prints none.
    pub(super) context: &'static str,
    /// `SIGNATURE`, hexadecimal.
    pub(super) signature: &'static str,
}

/// Hexadecimal to octets.
pub(super) fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// Section 7.4, "Test Vectors for Ed448": all nine.
pub(super) const ED448: &[Vector] = &[
    Vector {
        name: "Blank",
        public_key: "\
            5fd7449b59b461fd2ce787ec616ad46a1da1342485a70e1f8a0ea75d80e96778edf124769b46c7061bd6783df1e50f6c\
            d1fa1abeafe8256180",
        message: "",
        context: "",
        signature: "\
            533a37f6bbe457251f023c0d88f976ae2dfb504a843e34d2074fd823d41a591f2b233f034f628281f2fd7a22ddd47d78\
            28c59bd0a21bfd3980ff0d2028d4b18a9df63e006c5d1c2d345b925d8dc00b4104852db99ac5c7cdda8530a113a0f4db\
            b61149f05a7363268c71d95808ff2e652600",
    },
    Vector {
        name: "1 octet",
        public_key: "\
            43ba28f430cdff456ae531545f7ecd0ac834a55d9358c0372bfa0c6c6798c0866aea01eb00742802b8438ea4cb82169c\
            235160627b4c3a9480",
        message: "\
            03",
        context: "",
        signature: "\
            26b8f91727bd62897af15e41eb43c377efb9c610d48f2335cb0bd0087810f4352541b143c4b981b7e18f62de8ccdf633\
            fc1bf037ab7cd779805e0dbcc0aae1cbcee1afb2e027df36bc04dcecbf154336c19f0af7e0a6472905e799f1953d2a0f\
            f3348ab21aa4adafd1d234441cf807c03a00",
    },
    Vector {
        name: "1 octet (with context)",
        public_key: "\
            43ba28f430cdff456ae531545f7ecd0ac834a55d9358c0372bfa0c6c6798c0866aea01eb00742802b8438ea4cb82169c\
            235160627b4c3a9480",
        message: "\
            03",
        context: "\
            666f6f",
        signature: "\
            d4f8f6131770dd46f40867d6fd5d5055de43541f8c5e35abbcd001b32a89f7d2151f7647f11d8ca2ae279fb842d60721\
            7fce6e042f6815ea000c85741de5c8da1144a6a1aba7f96de42505d7a7298524fda538fccbbb754f578c1cad10d54d0d\
            5428407e85dcbc98a49155c13764e66c3c00",
    },
    Vector {
        name: "11 octets",
        public_key: "\
            dcea9e78f35a1bf3499a831b10b86c90aac01cd84b67a0109b55a36e9328b1e365fce161d71ce7131a543ea4cb5f7e9f\
            1d8b00696447001400",
        message: "\
            0c3e544074ec63b0265e0c",
        context: "",
        signature: "\
            1f0a8888ce25e8d458a21130879b840a9089d999aaba039eaf3e3afa090a09d389dba82c4ff2ae8ac5cdfb7c55e94d5d\
            961a29fe0109941e00b8dbdeea6d3b051068df7254c0cdc129cbe62db2dc957dbb47b51fd3f213fb8698f064774250a5\
            028961c9bf8ffd973fe5d5c206492b140e00",
    },
    Vector {
        name: "12 octets",
        public_key: "\
            3ba16da0c6f2cc1f30187740756f5e798d6bc5fc015d7c63cc9510ee3fd44adc24d8e968b6e46e6f94d19b945361726b\
            d75e149ef09817f580",
        message: "\
            64a65f3cdedcdd66811e2915",
        context: "",
        signature: "\
            7eeeab7c4e50fb799b418ee5e3197ff6bf15d43a14c34389b59dd1a7b1b85b4ae90438aca634bea45e3a2695f1270f07\
            fdcdf7c62b8efeaf00b45c2c96ba457eb1a8bf075a3db28e5c24f6b923ed4ad747c3c9e03c7079efb87cb110d3a99861\
            e72003cbae6d6b8b827e4e6c143064ff3c00",
    },
    Vector {
        name: "13 octets",
        public_key: "\
            b3da079b0aa493a5772029f0467baebee5a8112d9d3a22532361da294f7bb3815c5dc59e176b4d9f381ca0938e13c6c0\
            7b174be65dfa578e80",
        message: "\
            64a65f3cdedcdd66811e2915e7",
        context: "",
        signature: "\
            6a12066f55331b6c22acd5d5bfc5d71228fbda80ae8dec26bdd306743c5027cb4890810c162c027468675ecf645a8317\
            6c0d7323a2ccde2d80efe5a1268e8aca1d6fbc194d3f77c44986eb4ab4177919ad8bec33eb47bbb5fc6e28196fd1caf5\
            6b4e7e0ba5519234d047155ac727a1053100",
    },
    Vector {
        name: "64 octets",
        public_key: "\
            df9705f58edbab802c7f8363cfe5560ab1c6132c20a9f1dd163483a26f8ac53a39d6808bf4a1dfbd261b099bb03b3fb5\
            0906cb28bd8a081f00",
        message: "\
            bd0f6a3747cd561bdddf4640a332461a4a30a12a434cd0bf40d766d9c6d458e5512204a30c17d1f50b5079631f64eb31\
            12182da3005835461113718d1a5ef944",
        context: "",
        signature: "\
            554bc2480860b49eab8532d2a533b7d578ef473eeb58c98bb2d0e1ce488a98b18dfde9b9b90775e67f47d4a1c3482058\
            efc9f40d2ca033a0801b63d45b3b722ef552bad3b4ccb667da350192b61c508cf7b6b5adadc2c8d9a446ef003fb05cba\
            5f30e88e36ec2703b349ca229c2670833900",
    },
    Vector {
        name: "256 octets",
        public_key: "\
            79756f014dcfe2079f5dd9e718be4171e2ef2486a08f25186f6bff43a9936b9bfe12402b08ae65798a3d81e22e9ec80e\
            7690862ef3d4ed3a00",
        message: "\
            15777532b0bdd0d1389f636c5f6b9ba734c90af572877e2d272dd078aa1e567cfa80e12928bb542330e8409f31745041\
            07ecd5efac61ae7504dabe2a602ede89e5cca6257a7c77e27a702b3ae39fc769fc54f2395ae6a1178cab4738e543072f\
            c1c177fe71e92e25bf03e4ecb72f47b64d0465aaea4c7fad372536c8ba516a6039c3c2a39f0e4d832be432dfa9a706a6\
            e5c7e19f397964ca4258002f7c0541b590316dbc5622b6b2a6fe7a4abffd96105eca76ea7b98816af0748c10df048ce0\
            12d901015a51f189f3888145c03650aa23ce894c3bd889e030d565071c59f409a9981b51878fd6fc110624dcbcde0bf7\
            a69ccce38fabdf86f3bef6044819de11",
        context: "",
        signature: "\
            c650ddbb0601c19ca11439e1640dd931f43c518ea5bea70d3dcde5f4191fe53f00cf966546b72bcc7d58be2b9badef28\
            743954e3a44a23f880e8d4f1cfce2d7a61452d26da05896f0a50da66a239a8a188b6d825b3305ad77b73fbac0836ecc6\
            0987fd08527c1a8e80d5823e65cafe2a3d00",
    },
    Vector {
        name: "1023 octets",
        public_key: "\
            a81b2e8a70a5ac94ffdbcc9badfc3feb0801f258578bb114ad44ece1ec0e799da08effb81c5d685c0c56f64eecaef8cd\
            f11cc38737838cf400",
        message: "\
            6ddf802e1aae4986935f7f981ba3f0351d6273c0a0c22c9c0e8339168e675412a3debfaf435ed651558007db4384b650\
            fcc07e3b586a27a4f7a00ac8a6fec2cd86ae4bf1570c41e6a40c931db27b2faa15a8cedd52cff7362c4e6e23daec0fbc\
            3a79b6806e316efcc7b68119bf46bc76a26067a53f296dafdbdc11c77f7777e972660cf4b6a9b369a6665f02e0cc9b6e\
            dfad136b4fabe723d2813db3136cfde9b6d044322fee2947952e031b73ab5c603349b307bdc27bc6cb8b8bbd7bd32321\
            9b8033a581b59eadebb09b3c4f3d2277d4f0343624acc817804728b25ab797172b4c5c21a22f9c7839d64300232eb66e\
            53f31c723fa37fe387c7d3e50bdf9813a30e5bb12cf4cd930c40cfb4e1fc622592a49588794494d56d24ea4b40c89fc0\
            596cc9ebb961c8cb10adde976a5d602b1c3f85b9b9a001ed3c6a4d3b1437f52096cd1956d042a597d561a596ecd3d173\
            5a8d570ea0ec27225a2c4aaff26306d1526c1af3ca6d9cf5a2c98f47e1c46db9a33234cfd4d81f2c98538a09ebe76998\
            d0d8fd25997c7d255c6d66ece6fa56f11144950f027795e653008f4bd7ca2dee85d8e90f3dc315130ce2a00375a318c7\
            c3d97be2c8ce5b6db41a6254ff264fa6155baee3b0773c0f497c573f19bb4f4240281f0b1f4f7be857a4e59d416c06b4\
            c50fa09e1810ddc6b1467baeac5a3668d11b6ecaa901440016f389f80acc4db977025e7f5924388c7e340a732e554440\
            e76570f8dd71b7d640b3450d1fd5f0410a18f9a3494f707c717b79b4bf75c98400b096b21653b5d217cf3565c9597456\
            f70703497a078763829bc01bb1cbc8fa04eadc9a6e3f6699587a9e75c94e5bab0036e0b2e711392cff0047d0d6b05bd2\
            a588bc109718954259f1d86678a579a3120f19cfb2963f177aeb70f2d4844826262e51b80271272068ef5b3856fa8535\
            aa2a88b2d41f2a0e2fda7624c2850272ac4a2f561f8f2f7a318bfd5caf9696149e4ac824ad3460538fdc25421beec2cc\
            6818162d06bbed0c40a387192349db67a118bada6cd5ab0140ee273204f628aad1c135f770279a651e24d8c14d75a605\
            9d76b96a6fd857def5e0b354b27ab937a5815d16b5fae407ff18222c6d1ed263be68c95f32d908bd895cd76207ae7264\
            87567f9a67dad79abec316f683b17f2d02bf07e0ac8b5bc6162cf94697b3c27cd1fea49b27f23ba2901871962506520c\
            392da8b6ad0d99f7013fbc06c2c17a569500c8a7696481c1cd33e9b14e40b82e79a5f5db82571ba97bae3ad3e0479515\
            bb0e2b0f3bfcd1fd33034efc6245eddd7ee2086ddae2600d8ca73e214e8c2b0bdb2b047c6a464a562ed77b73d2d841c4\
            b34973551257713b753632efba348169abc90a68f42611a40126d7cb21b58695568186f7e569d2ff0f9e745d0487dd2e\
            b997cafc5abf9dd102e62ff66cba87",
        context: "",
        signature: "\
            e301345a41a39a4d72fff8df69c98075a0cc082b802fc9b2b6bc503f926b65bddf7f4c8f1cb49f6396afc8a70abe6d8a\
            ef0db478d4c6b2970076c6a0484fe76d76b3a97625d79f1ce240e7c576750d295528286f719b413de9ada3e8eb78ed57\
            3603ce30d8bb761785dc30dbc320869e1a00",
    },
];

/// Section 7.5, "Test Vectors for Ed448ph": both.
pub(super) const ED448PH: &[Vector] = &[
    Vector {
        name: "TEST abc",
        public_key: "\
            259b71c19f83ef77a7abd26524cbdb3161b590a48f7d17de3ee0ba9c52beb743c09428a131d6b1b57303d90d8132c276\
            d5ed3d5d01c0f53880",
        message: "\
            616263",
        context: "",
        signature: "\
            822f6901f7480f3d5f562c592994d9693602875614483256505600bbc281ae381f54d6bce2ea911574932f52a4e6cadd\
            78769375ec3ffd1b801a0d9b3f4030cd433964b6457ea39476511214f97469b57dd32dbc560a9a94d00bff07620464a3\
            ad203df7dc7ce360c3cd3696d9d9fab90f00",
    },
    Vector {
        name: "TEST abc (with context)",
        public_key: "\
            259b71c19f83ef77a7abd26524cbdb3161b590a48f7d17de3ee0ba9c52beb743c09428a131d6b1b57303d90d8132c276\
            d5ed3d5d01c0f53880",
        message: "\
            616263",
        context: "\
            666f6f",
        signature: "\
            c32299d46ec8ff02b54540982814dce9a05812f81962b649d528095916a2aa481065b1580423ef927ecf0af5888f90da\
            0f6a9a85ad5dc3f280d91224ba9911a3653d00e484e2ce232521481c8658df304bb7745a73514cdb9bf3e15784ab7128\
            4f8d0704a608c54a6b62d97beb511d132100",
    },
];
