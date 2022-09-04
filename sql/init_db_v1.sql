
CREATE TABLE `db_ver` (
    `version` mediumint unsigned NOT NULL COMMENT 'Database version',
    PRIMARY KEY (`version`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `user` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT,
    `user` varchar(128) NOT NULL,
    PRIMARY KEY (`id`),
    UNIQUE KEY `user` (`user`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `email` (
     `id` bigint unsigned NOT NULL AUTO_INCREMENT,
     `email` varchar(512) NOT NULL,
     PRIMARY KEY (`id`),
     UNIQUE (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `record` (
    `id` bigint unsigned NOT NULL AUTO_INCREMENT,
    `user_id` bigint unsigned NOT NULL,
    `uid` varchar(256) NOT NULL,
    `guid` varchar(256) NOT NULL,
    `mailbox` varchar(256) NOT NULL,
    `dt_sent` DATETIME NOT NULL,
    `tz_sent` DECIMAL(4) SIGNED NOT NULL,
    `dt_recv` DATETIME NOT NULL,
    `dt_saved` DATETIME NOT NULL,
    `size` bigint unsigned NOT NULL,
    `mail_subj` TEXT,
    `outbound` BOOL NOT NULL,
    `mail_from` bigint unsigned,
    UNIQUE (user_id,uid,mailbox),
    UNIQUE (guid),
    PRIMARY KEY (`id`),
    CONSTRAINT `fk_record_user_id`
        FOREIGN KEY (user_id) REFERENCES user (id)
            ON DELETE CASCADE
            ON UPDATE RESTRICT,
    CONSTRAINT `fk_record_email_id`
        FOREIGN KEY (mail_from) REFERENCES email (id)
            ON DELETE CASCADE
            ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `mail_to` (
    `record_id` bigint unsigned NOT NULL,
    `email_id` bigint unsigned NOT NULL,
    PRIMARY KEY (record_id, email_id),
    CONSTRAINT `fk_mail_to_record_id`
       FOREIGN KEY (record_id) REFERENCES record (id)
           ON DELETE CASCADE
           ON UPDATE RESTRICT,
    CONSTRAINT `fk_mail_to_email_id`
       FOREIGN KEY (email_id) REFERENCES email (id)
           ON DELETE CASCADE
           ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `mail_cc` (
    `record_id` bigint unsigned NOT NULL,
    `email_id` bigint unsigned NOT NULL,
    PRIMARY KEY (record_id, email_id),
    CONSTRAINT `fk_mail_cc_record_id`
       FOREIGN KEY (record_id) REFERENCES record (id)
           ON DELETE CASCADE
           ON UPDATE RESTRICT,
    CONSTRAINT `fk_mail_cc_email_id`
       FOREIGN KEY (email_id) REFERENCES email (id)
           ON DELETE CASCADE
           ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `mail_bcc` (
    `record_id` bigint unsigned NOT NULL,
    `email_id` bigint unsigned NOT NULL,
    PRIMARY KEY (record_id, email_id),
    CONSTRAINT `fk_mail_bcc_record_id`
       FOREIGN KEY (record_id) REFERENCES record (id)
           ON DELETE CASCADE
           ON UPDATE RESTRICT,
    CONSTRAINT `fk_mail_bcc_email_id`
       FOREIGN KEY (email_id) REFERENCES email (id)
           ON DELETE CASCADE
           ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `header` (
    `record_id` bigint unsigned NOT NULL,
    `seq` mediumint unsigned NOT NULL,
    `name` varchar(256) NOT NULL,
    `value` TEXT NOT NULL,
    PRIMARY KEY (record_id, seq),
    CONSTRAINT `fk_hdr_record_id`
        FOREIGN KEY (record_id) REFERENCES record (id)
        ON DELETE CASCADE
        ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `imap_flag` (
      `record_id` bigint unsigned NOT NULL,
      `name` varchar(256) NOT NULL,
      PRIMARY KEY (record_id, name),
      CONSTRAINT `fk_if_record_id`
          FOREIGN KEY (record_id) REFERENCES record (id)
              ON DELETE CASCADE
              ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `mail_stats` (
     `email_id` bigint unsigned NOT NULL,
     `user_id` bigint unsigned NOT NULL,
     `referenced` int unsigned DEFAULT 0,
     `inbound` int unsigned DEFAULT 0,
     `outbound` int unsigned DEFAULT 0,     # outbound
     `seen` int unsigned DEFAULT 0,         # inbound seen
     `spam` int unsigned DEFAULT 0,         # inbound marked spam
     PRIMARY KEY (email_id, user_id),
    CONSTRAINT `fk_ms_email_id`
         FOREIGN KEY (email_id) REFERENCES email (id)
         ON DELETE CASCADE
         ON UPDATE RESTRICT,
    CONSTRAINT `fk_ms_user_id`
             FOREIGN KEY (user_id) REFERENCES user (id)
             ON DELETE CASCADE
             ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `mail_name` (
    `email_id` bigint unsigned NOT NULL,
    `name` varchar(512) NOT NULL,
    PRIMARY KEY (email_id,name),
    CONSTRAINT `fk_mn_email_id`
        FOREIGN KEY (email_id) REFERENCES email (id)
            ON DELETE CASCADE
            ON UPDATE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;


# CREATE VIEW whitelist
#     AS SELECT DISTINCT rec.user_id, rec.mail_from FROM record as rec, imap_flag as flag
#         WHERE rec.id=flag.record_id and flag.name='\\Answered'

INSERT INTO db_ver (version) VALUES(1);
